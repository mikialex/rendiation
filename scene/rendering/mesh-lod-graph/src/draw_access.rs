use crate::*;

only_vertex!(DrawMeshletIndex, u32);

pub struct MeshletBatchDrawData {
  pub meshlets_idx: StorageBufferReadonlyDataView<[u32]>,
  pub scene_model_idx: StorageBufferReadonlyDataView<[u32]>,
  pub command: DrawCommand,
}

impl ShaderPassBuilder for MeshletBatchDrawData {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.meshlets_idx);
    ctx.binding.bind(&self.scene_model_idx);
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
      scene_model_idx: ShaderReadonlyPtrOf<[u32]>,
    }

    impl IndirectBatchInvocationSource for MeshletBatchDrawInvocation {
      fn current_invocation_scene_model_id(&self, builder: &mut ShaderVertexBuilder) -> Node<u32> {
        let draw_id = builder.query::<VertexInstanceIndex>();
        self.scene_model_idx.index(draw_id).load()
      }

      fn extra_register(&self, builder: &mut ShaderVertexBuilder) {
        let draw_id = builder.query::<VertexInstanceIndex>();
        let meshlet_idx = self.meshlet_idx.index(draw_id).load();
        builder.register::<DrawMeshletIndex>(meshlet_idx);
      }
    }

    Box::new(MeshletBatchDrawInvocation {
      meshlet_idx: binding.bind_by(&self.meshlets_idx),
      scene_model_idx: binding.bind_by(&self.scene_model_idx),
    })
  }

  fn draw_command(&self) -> DrawCommand {
    self.command.clone()
  }
}

/// The implementation of Logical Mesh [RenderComponent]
pub struct MeshletGPURenderData {
  pub meshlet_metadata: AbstractReadonlyStorageBuffer<[MeshletMetaData]>,
  pub position_buffer: AbstractReadonlyStorageBuffer<[u32]>,
  pub index_buffer: AbstractReadonlyStorageBuffer<[u32]>,
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
        (mesh_meta.position_offset().load() + vertex_index) * val(3),
        StructLayoutTarget::Packed,
      );

      builder.register::<GeometryPosition>(position);
      builder.register::<GeometryUVChannel<0>>(zeroed_val());
      builder.register::<GeometryNormal>(val(Vec3::new(0., 1., 0.)));

      builder.primitive_state.topology = PrimitiveTopology::TriangleList;
    })
  }
}

impl ShaderPassBuilder for MeshletGPURenderData {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.pass.set_index_buffer_by_buffer_resource_view(
      &self.index_buffer.get_gpu_buffer_view().unwrap(),
      IndexFormat::Uint32,
    );

    ctx.binding.bind(&self.position_buffer);
    ctx.binding.bind(&self.meshlet_metadata);
  }
}
