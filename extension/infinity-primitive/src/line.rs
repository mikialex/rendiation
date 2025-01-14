use crate::*;

pub const LINE_DRAW_CMD: DrawCommand = DrawCommand::Array {
  vertices: 0..2,
  instances: 0..1,
};

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderLine {
  pub point: Vec3<f32>,
  pub direction: Vec3<f32>,
}

pub struct InfinityShaderLineEffect<'a> {
  pub line: &'a UniformBufferCachedDataView<ShaderLine>,
  pub camera: &'a dyn RenderComponent,
}

impl<'a> ShaderHashProvider for InfinityShaderLineEffect<'a> {
  shader_hash_type_id! {InfinityShaderPlaneEffect<'static>}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.camera.hash_pipeline(hasher);
  }
}
impl<'a> ShaderPassBuilder for InfinityShaderLineEffect<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.camera.setup_pass(ctx);
    ctx.binding.bind(self.line);
  }
}

impl<'a> GraphicsShaderProvider for InfinityShaderLineEffect<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.camera.build(builder);

    builder.vertex(|builder, bind| {
      let vertex_index = builder.query::<VertexIndex>();
      let view_proj = builder.query::<CameraViewProjectionMatrix>();
      let view_proj_inv = builder.query::<CameraViewProjectionInverseMatrix>();

      let line = bind.bind_by(&self.line).load().expand();
      let origin_in_ndc: Node<Vec3<f32>> = view_proj * line.point;
      let direct_in_ndc: Node<Vec3<f32>> = view_proj_inv.transpose() * line.direction;

      let position = val(Vec3::zero()).make_local_var();
      switch_by(vertex_index)
        .case(0, || {
          let near_plane = ENode::<ShaderPlane> {
            normal: val(Vec3::new(0., 0., 1.)),
            constant: val(1.),
          };
          let p = ray_plane_intersect(origin_in_ndc, direct_in_ndc, near_plane).xyz();
          position.store(p);
        })
        .end_with_default(|| {
          let far_plane = ENode::<ShaderPlane> {
            normal: val(Vec3::new(0., 0., 1.)),
            constant: val(-1.),
          };
          let p = ray_plane_intersect(origin_in_ndc, -direct_in_ndc, far_plane).xyz();
          position.store(p);
        });

      builder.register::<ClipPosition>((position.load(), val(1.)));

      builder.primitive_state = PrimitiveState {
        topology: PrimitiveTopology::LineList,
        ..Default::default()
      };
    });
  }
}
