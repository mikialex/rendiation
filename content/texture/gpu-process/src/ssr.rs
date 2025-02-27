use rendiation_shader_library::{shader_uv_space_to_world_space, shader_world_space_to_uv_space};

use crate::*;

#[derive(Clone, Copy, ShaderStruct)]
pub struct NaiveScreenSpaceReflectionConfig {
  pub trace_check_step_count: u32,
  pub max_distance: f32,
  pub thickness: f32,
  pub infinite_thick: Bool,
}

pub fn screen_space_reflection(
  config: ENode<NaiveScreenSpaceReflectionConfig>,
  uv: Node<Vec2<f32>>,
  sampler: BindingNode<ShaderSampler>,
  depth: BindingNode<ShaderDepthTexture2D>,
  normal: BindingNode<ShaderTexture2D>,
  radiance: BindingNode<ShaderTexture2D>,
  camera_position: Node<Vec3<f32>>,
  reproject: ENode<ReprojectInfo>,
  texel: Node<Vec2<f32>>,
) -> Node<Vec3<f32>> {
  let d = depth.sample(sampler, uv);
  let world_position: Node<Vec3<f32>> =
    shader_uv_space_to_world_space(reproject.current_camera_view_projection_inv, uv, d);
  let world_surface_normal = normal.sample(sampler, uv).xyz();

  let camera_to_surface = world_position - camera_position;
  let trace_ray_direction = camera_to_surface.reflect(world_surface_normal);

  let ray_end_point = world_position + trace_ray_direction * config.max_distance;

  let ray_end_point_in_uv =
    shader_world_space_to_uv_space(reproject.current_camera_view_projection, ray_end_point).0;
  let uv_space_trace_dir = ray_end_point_in_uv - uv;
  let uv_space_trace_length = uv_space_trace_dir.length();
  let uv_space_trace_step_length = uv_space_trace_length / config.trace_check_step_count.into_f32();
  let uv_space_trace_step_dir = uv_space_trace_dir / uv_space_trace_step_length.splat();

  let current_test_point_var = uv.make_local_var();
  let sampled_radiance = zeroed_val::<Vec3<f32>>().make_local_var();
  loop_by(|cx| {
    let previous_test_point = current_test_point_var.load();
    let current_test_point = previous_test_point + uv_space_trace_step_dir;

    #[rustfmt::skip]
    let out_of_screen = current_test_point.x().less_than(0.0).or(current_test_point.x().greater_than(1.0))
      .or(current_test_point.y().less_than(0.0).or(current_test_point.y().greater_than(1.0)));
    if_by(out_of_screen, || cx.do_break());

    let infinite_thick = config.infinite_thick.into_bool();
    let accept_test = infinite_thick.make_local_var();
    if_by(infinite_thick.not(), || {
      let current_test_depth = depth.sample(sampler, current_test_point);
      let test_world_position = shader_uv_space_to_world_space(
        reproject.current_camera_view_projection_inv,
        current_test_point,
        current_test_depth,
      );
      let away = point_to_line_distance(test_world_position, world_position, ray_end_point);

      let neighbor_uv = current_test_point + texel;
      let neighbor_depth = depth.sample(sampler, neighbor_uv);
      let neighbor_world_position = shader_uv_space_to_world_space(
        reproject.current_camera_view_projection_inv,
        neighbor_uv,
        neighbor_depth,
      );
      let min_thickness = (test_world_position - neighbor_world_position).length();
      let thickness = config.thickness.max(min_thickness);

      accept_test.store(away.less_than(thickness));
    });

    if_by(accept_test.load(), || {
      sampled_radiance.store(radiance.sample(sampler, current_test_point).xyz());
      cx.do_break();
    });

    current_test_point_var.store(current_test_point);
  });
  sampled_radiance.load()
}

/// https://mathworld.wolfram.com/Point-LineDistance3-Dimensional.html
fn point_to_line_distance(
  point: Node<Vec3<f32>>,
  line_point_a: Node<Vec3<f32>>,
  line_point_b: Node<Vec3<f32>>,
) -> Node<f32> {
  (point - line_point_a).cross(point - line_point_b).length()
    / (line_point_b - line_point_a).length()
}
