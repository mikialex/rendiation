use rendiation_mesh_generator::*;

use crate::*;

pub fn use_scene_spotlight_helper(cx: &mut ViewerCx) {
  let (cx, enabled) = cx.use_plain_state::<bool>();

  if let ViewerCxStage::Gui { egui_ctx, global } = &mut cx.stage {
    let opened = global.features.entry("light helper").or_insert(false);

    egui::Window::new("Light Helper")
      .open(opened)
      .default_size((100., 100.))
      .vscroll(true)
      .show(egui_ctx, |ui| {
        ui.checkbox(enabled, "enabled");
      });
  }

  if *enabled {
    cx.scope(|cx| {
      let world_mat = use_global_node_world_mat_view(cx);

      let helper_mesh_lines = world_mat.map_only_spawn_stage(|world_mat| {
        let half_cone_angle = get_db_view::<SpotLightHalfConeAngle>();
        let half_penumbra_angle = get_db_view::<SpotLightHalfPenumbraAngle>();
        let cutoff = get_db_view::<SpotLightCutOffDistance>();
        let light_ref_node = get_db_view::<SpotLightRefNode>();

        let mut line_buffer = Vec::new();

        half_cone_angle
          .iter_key_value()
          .for_each(|(id, half_cone_angle)| {
            let node_id = light_ref_node.access(&id).unwrap().unwrap();
            create_debug_line_mesh(
              &mut line_buffer,
              half_cone_angle,
              half_penumbra_angle.access(&id).unwrap(),
              cutoff.access(&id).unwrap(),
              world_mat.access(&node_id).unwrap().into_f32(),
            )
          });
        line_buffer.into()
      });

      use_immediate_helper_model(cx, helper_mesh_lines);
    })
  }
}

fn create_debug_line_mesh(
  lines: &mut LineBuffer,
  half_angle: f32,
  half_penumbra: f32,
  cutoff: f32,
  world_mat: Mat4<f32>,
) {
  fn build_cone(half_angle: f32, cutoff: f32, world_mat: Mat4<f32>, lines: &mut LineBuffer) {
    let radius = half_angle.tan() * cutoff;
    let angle_outlines_ends = [
      Vec3::new(-radius, 0., -cutoff),
      Vec3::new(radius, 0., -cutoff),
      Vec3::new(0., -radius, -cutoff),
      Vec3::new(0., radius, -cutoff),
    ];

    lines.extend(
      angle_outlines_ends
        .into_iter()
        .map(|ends| [world_mat.position(), world_mat * ends]),
    );

    let circle = create_circle(radius, cutoff).transform3d_by(world_mat);

    let step_count = 32;
    let step_size = 1.0 / step_count as f32;
    for i in 0..step_count {
      let start = circle.position(step_size * i as f32);
      let end = circle.position(step_size * (i + 1) as f32);
      lines.push([start, end]);
    }
  }

  build_cone(half_angle, cutoff, world_mat, lines);
  build_cone(half_penumbra, cutoff, world_mat, lines);
}

fn create_circle(radius: f32, offset: f32) -> impl ParametricCurve3D {
  UnitCircle
    .transform_by(Mat3::scale(Vec2::splat(radius)))
    .embed_to_surface(ParametricPlane.transform3d_by(Mat4::translate((0., 0., -offset))))
}
