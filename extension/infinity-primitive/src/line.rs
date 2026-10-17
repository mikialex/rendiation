use crate::*;

pub const LINE_DRAW_CMD: DrawCommand = DrawCommand::Array {
  vertices: 0..2,
  instances: 0..1,
};

/// a ray start from the point and extend infinitely along the direction. an infinite line can be
/// drawn by two rays with the same point and opposite direction.
#[repr(C)]
#[shader_struct(std140)]
#[derive(Copy, Clone)]
pub struct ShaderLine {
  pub point: HighPrecisionTranslationUniform,
  pub direction: Vec3<f32>,
}

impl ShaderLine {
  pub fn new(point: Vec3<f64>, direction: Vec3<f32>) -> Self {
    Self {
      point: into_hpt(point).into_uniform(),
      direction,
      ..Zeroable::zeroed()
    }
  }
}

pub struct InfinityShaderLineEffect<'a> {
  pub line: &'a UniformBufferCachedDataView<ShaderLine>,
  pub camera: &'a dyn RenderComponent,
}

impl ShaderHashProvider for InfinityShaderLineEffect<'_> {
  shader_hash_type_id! {InfinityShaderLineEffect<'static>}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.camera.hash_pipeline(hasher);
  }
}
impl ShaderPassBuilder for InfinityShaderLineEffect<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.camera.setup_pass(ctx);
    ctx.binding.bind(self.line);
  }
}

impl GraphicsShaderProvider for InfinityShaderLineEffect<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.camera.build(builder);

    builder.vertex(|builder, bind| {
      let line = bind.bind_by(&self.line).load().expand();
      let camera_position = builder.query::<CameraWorldPositionHP>();
      let origin = hpt_sub_hpt(hpt_uniform_to_hpt(line.point), camera_position);

      // the second vertex is the point at infinity along the direction(w = 0). the linear
      // interpolation between (origin, 1) and (direction, 0) in homogeneous space covers exactly
      // the whole ray, so the hardware clipping produces the correct result even if the origin is
      // behind the camera or the ray is parallel to the near plane.
      let is_origin = builder.query::<VertexIndex>().equals(0);
      let position = is_origin.select((origin, val(1.)), (line.direction, val(0.)));

      let clip_position = compute_jittered_clip_space_position(builder, position);
      builder.register::<ClipPosition>(clip_position);

      *builder.primitive_state() = PrimitiveState {
        topology: PrimitiveTopology::LineList,
        ..Default::default()
      };
    });
  }
}
