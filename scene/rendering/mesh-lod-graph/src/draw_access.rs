use crate::*;

only_vertex!(DrawMeshletIndex, u32);

pub struct MeshletBatchDrawData {
  pub meshlets_idx: StorageBufferDataView<[u32]>,
  pub command: DrawCommand,
}

impl ShaderPassBuilder for MeshletBatchDrawData {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.meshlets_idx);
  }
}

impl ShaderHashProvider for MeshletBatchDrawData {
  shader_hash_type_id! {}
}

impl IndirectDrawProvider for MeshletBatchDrawData {
  fn create_indirect_invocation_source(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn IndirectBatchInvocationSource> {
    struct MeshletBatchDrawInvocation {
      meshlet_idx: ShaderReadonlyPtrOf<[u32]>,
    }

    impl IndirectBatchInvocationSource for MeshletBatchDrawInvocation {
      fn current_invocation_scene_model_id(&self, builder: &ShaderVertexBuilder) -> Node<u32> {
        builder.query::<VertexInstanceIndex>()
      }

      fn extra_register(&self, builder: &mut ShaderVertexBuilder) {
        // todo, inject meshlet index
        todo!()
      }
    }

    Box::new(MeshletBatchDrawInvocation {
      meshlet_idx: binding.bind_by(&self.meshlets_idx.clone().into_readonly_view()),
    })
  }

  fn draw_command(&self) -> DrawCommand {
    self.command.clone()
  }
}

/// The implementation of Logical Mesh [RenderComponent]
pub struct MeshletGPURenderData {
  pub meshlet_metadata: StorageBufferReadonlyDataView<[MeshletMetaData]>,
  pub position_buffer: StorageBufferReadonlyDataView<[u32]>,
  pub index_buffer: StorageBufferReadonlyDataView<[u32]>,
}

impl ShaderHashProvider for MeshletGPURenderData {
  shader_hash_type_id! {}
}

impl GraphicsShaderProvider for MeshletGPURenderData {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let position_buffer = binding.bind_by(&self.position_buffer);
      let mesh_src_data = binding.bind_by(&self.meshlet_metadata);

      let mesh_index = builder.query::<DrawMeshletIndex>();
      let vertex_index = builder.query::<VertexIndex>();

      let mesh_meta = mesh_src_data.index(mesh_index);
      let position = Node::<Vec3<f32>>::load_from_u32_buffer(
        &position_buffer,
        mesh_meta.position_offset().load() + vertex_index * val(3),
        StructLayoutTarget::Packed,
      );

      builder.register::<GeometryPosition>(position);

      builder.primitive_state.topology = PrimitiveTopology::TriangleList;
    })
  }
}

impl ShaderPassBuilder for MeshletGPURenderData {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx
      .pass
      .set_index_buffer_by_buffer_resource_view(&self.index_buffer, IndexFormat::Uint32);

    ctx.binding.bind(&self.position_buffer);
    ctx.binding.bind(&self.meshlet_metadata);
  }
}
