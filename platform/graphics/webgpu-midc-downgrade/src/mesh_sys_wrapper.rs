use crate::*;

pub struct MidcDowngradeWrapperForIndirectMeshSystem<T> {
  pub mesh_system: T,
  pub enable_downgrade: bool,
  pub index: Option<AbstractReadonlyStorageBuffer<[u32]>>,
}

impl<T: ShaderHashProvider + 'static> ShaderHashProvider
  for MidcDowngradeWrapperForIndirectMeshSystem<T>
{
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.mesh_system.hash_pipeline(hasher);
    self.enable_downgrade.hash(hasher);
  }
}

impl<T> GraphicsShaderProvider for MidcDowngradeWrapperForIndirectMeshSystem<T>
where
  T: GraphicsShaderProvider,
{
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|vertex, binding| {
      // here we override the builtin
      if self.enable_downgrade {
        if let Some(index) = &self.index {
          let vertex_real_index = vertex.query::<VertexIndexForMIDCDowngrade>();
          let index_pool = binding.bind_by(index);
          let index = index_pool.index(vertex_real_index).load();
          vertex.register::<VertexIndex>(index);
        } else {
          let relative = vertex.query::<VertexIndexForMIDCDowngradeRelative>();
          vertex.register::<VertexIndex>(relative);
        }
      }
    });
    self.mesh_system.build(builder);
  }
}

impl<T: ShaderPassBuilder> ShaderPassBuilder for MidcDowngradeWrapperForIndirectMeshSystem<T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    if let Some(index) = &self.index {
      // when midc downgrade enabled, the index multi draw will be downgraded into single none index draw,
      // so we use storage binding for index buffer
      if self.enable_downgrade {
        ctx.binding.bind(index);
      } else {
        let index = index.get_gpu_buffer_view().unwrap();
        ctx
          .pass
          .set_index_buffer_by_buffer_resource_view(&index, IndexFormat::Uint32);
      }
    }
    self.mesh_system.setup_pass(ctx);
  }
}
