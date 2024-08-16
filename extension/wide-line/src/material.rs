use crate::*;

#[repr(C)]
#[derive(Clone)]
pub struct WideLineMaterial {
  pub width: f32,
}

impl WideLineMaterial {
  pub fn new(width: f32) -> Self {
    Self { width }
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct WideLineMaterialUniform {
  pub width: f32,
}

pub struct WideLineMaterialGPU<'a> {
  uniform: &'a UniformBufferDataView<WideLineMaterialUniform>,
}

impl<'a> ShaderHashProvider for WideLineMaterialGPU<'a> {
  shader_hash_type_id! {WideLineMaterialGPU<'static>}
}

impl<'a> ShaderPassBuilder for WideLineMaterialGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
  }
}

impl<'a> GraphicsShaderProvider for WideLineMaterialGPU<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, binding| {
      let uv = builder.query::<GeometryUV>()?;
      let color_with_alpha = builder.query::<GeometryColorWithAlpha>()?;
      let material = binding.bind_by(&self.uniform).load().expand();

      // todo move to shape post build
      let vertex_position = wide_line_vertex(
        builder.query::<CameraProjectionMatrix>()?,
        builder.query::<CameraViewMatrix>()?,
        builder.query::<WorldMatrix>()?,
        builder.query::<WideLineStart>()?,
        builder.query::<WideLineEnd>()?,
        builder.query::<GeometryPosition>()?,
        builder.query::<RenderBufferSize>()?,
        material.width,
      );

      builder.register::<ClipPosition>(vertex_position);
      builder.set_vertex_out::<FragmentUv>(uv);
      builder.set_vertex_out::<FragmentColorAndAlpha>(color_with_alpha);
      Ok(())
    })?;

    builder.fragment(|builder, _| {
      let uv = builder.query::<FragmentUv>()?;
      let color = builder.query::<FragmentColorAndAlpha>()?;

      if_by(discard_round_corner(uv), || {
        builder.discard();
      });

      builder.register::<DefaultDisplay>(color);
      Ok(())
    })
  }
}

fn wide_line_vertex(
  projection: Node<Mat4<f32>>,
  view: Node<Mat4<f32>>,
  world_matrix: Node<Mat4<f32>>,
  wide_line_start: Node<Vec3<f32>>,
  wide_line_end: Node<Vec3<f32>>,
  position: Node<Vec3<f32>>,
  view_size: Node<Vec2<f32>>,
  width: Node<f32>,
) -> Node<Vec4<f32>> {
  let wide_line_start = vec4_node((wide_line_start, val(1.0)));
  let wide_line_end = vec4_node((wide_line_end, val(1.0)));
  // camera space
  let start = view * world_matrix * wide_line_start;
  let end = view * world_matrix * wide_line_end;

  let aspect = view_size.x() / view_size.y();

  // clip space
  let clip_start = projection * start;
  let clip_end = projection * end;

  // ndc space
  let ndc_start = clip_start.xy() / clip_start.w().splat();
  let ndc_end = clip_end.xy() / clip_end.w().splat();

  // direction
  let dir = ndc_end - ndc_start;

  // account for clip-space aspect ratio
  let dir = vec2_node((dir.x() * aspect, dir.y()));
  let dir = dir.normalize();

  // perpendicular to dir
  let offset = vec2_node((dir.y(), -dir.x()));

  // undo aspect ratio adjustment
  let dir = vec2_node((dir.x() / aspect, dir.y()));
  let offset = vec2_node((offset.x() / aspect, offset.y()));
  let offset = offset.make_local_var();

  // sign flip
  if_by(position.x().less_than(0.), || {
    offset.store(-offset.load());
  });

  // end caps
  if_by(position.y().less_than(0.), || {
    offset.store(offset.load() - dir);
  });

  if_by(position.y().greater_than(1.), || {
    offset.store(offset.load() + dir);
  });

  let mut offset = offset.load();

  // adjust for width
  offset *= width.splat();
  // adjust for clip-space to screen-space conversion // maybe resolution should be based on
  // viewport ...
  offset /= view_size.y().splat();

  // select end
  let clip = position.y().less_than(0.5).select(clip_start, clip_end);

  // back to clip space
  offset = offset * clip.w();
  (clip.xy() + offset, clip.zw()).into()
}

fn discard_round_corner(uv: Node<Vec2<f32>>) -> Node<bool> {
  let a = uv.x();
  let b = uv.y() + uv.y().greater_than(0.).select(-1., 1.);
  let len2 = a * a + b * b;

  uv.y().abs().greater_than(1.).and(len2.greater_than(1.))
}
