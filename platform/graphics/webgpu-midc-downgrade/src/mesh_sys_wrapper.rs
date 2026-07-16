use crate::*;

pub struct MidcDowngradeWrapperForIndirectMeshSystem {
  pub index: Option<AbstractReadonlyStorageBuffer<[u32]>>,
}

impl ShaderHashProvider for MidcDowngradeWrapperForIndirectMeshSystem {
  shader_hash_type_id! {}
}

impl GraphicsShaderProvider for MidcDowngradeWrapperForIndirectMeshSystem {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|vertex, binding| {
      // here we override the builtin
      if let Some(index) = &self.index {
        let vertex_real_index = vertex.query::<VertexIndexForMIDCDowngrade>();
        let index_pool = binding.bind_by(index);
        let index = index_pool.index(vertex_real_index).load();
        vertex.register::<VertexIndex>(index);
      } else {
        let relative = vertex.query::<VertexIndexForMIDCDowngradeRelative>();
        vertex.register::<VertexIndex>(relative);
      }
    });
  }
}

impl ShaderPassBuilder for MidcDowngradeWrapperForIndirectMeshSystem {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    if let Some(index) = &self.index {
      // when midc downgrade enabled, the index multi draw will be downgraded into single none index draw,
      // so we use storage binding for index buffer
      //
      // the subsequent mesh index buffer setting will still applied, but has no effect as we override the draw cmd.
      ctx.binding.bind(index);
    }
  }
}
