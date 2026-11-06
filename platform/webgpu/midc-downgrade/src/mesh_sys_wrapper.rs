use crate::*;

pub struct MidcDowngradeWrapperForIndirectMeshSystem {
  /// (index data pool, should_access_as_u16)
  pub index: Option<(AbstractReadonlyStorageBuffer<[u32]>, bool)>,
}

impl ShaderHashProvider for MidcDowngradeWrapperForIndirectMeshSystem {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.index.as_ref().map(|v| v.1));
  }
}

impl GraphicsShaderProvider for MidcDowngradeWrapperForIndirectMeshSystem {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|vertex, binding| {
      // here we override the builtin
      let relative = vertex.query::<VertexIndexForMIDCDowngradeRelativeInSubDraw>();
      if let Some((index, should_access_as_u16)) = &self.index {
        let base_index = vertex.query::<VertexIndexForMIDCDowngradeBaseIndex>();
        let index_pool = binding.bind_by(index);
        let index = if *should_access_as_u16 {
          let read = index_pool.index(base_index + relative / val(2)).load();
          // little-endian, the first u16 of each pair lives in the low half
          let low = read & val(0xffff);
          let high = read >> val(16);
          (relative % val(2)).equals(0).select(low, high)
        } else {
          index_pool.index(base_index + relative).load()
        };

        vertex.register::<VertexIndex>(index);
      } else {
        vertex.register::<VertexIndex>(relative);
      }
    });
  }
}

impl ShaderPassBuilder for MidcDowngradeWrapperForIndirectMeshSystem {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    if let Some((index, _)) = &self.index {
      // when midc downgrade enabled, the index multi draw will be downgraded into single none index draw,
      // so we use storage binding for index buffer
      //
      // the subsequent mesh index buffer setting will still applied, but has no effect as we override the draw cmd.
      ctx.binding.bind(index);
    }
  }
}
