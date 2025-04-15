use crate::*;

pub struct MeshletGPUDraw {
  // source: IndirectDrawProvider,
  pub mesh_src_data: StorageBufferReadonlyDataView<[MeshletMeshMetaData]>,
  pub position_buffer: StorageBufferReadonlyDataView<[u32]>,
  // _meshlet_buffer: StorageBufferReadonlyDataView<[u32]>, /* todo, debug visualization for level, group and meshlet */
  pub index_buffer: StorageBufferReadonlyDataView<[u32]>,
}

impl ShaderHashProvider for MeshletGPUDraw {
  shader_hash_type_id! {}
}

// todo
only_vertex!(MeshHandle, u32);

impl GraphicsShaderProvider for MeshletGPUDraw {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let position_buffer = binding.bind_by(&self.position_buffer);
      let mesh_src_data = binding.bind_by(&self.mesh_src_data);

      let mesh_index = builder.query::<MeshHandle>();
      let vertex_index = builder.query::<VertexIndex>();

      let mesh_meta = mesh_src_data.index(mesh_index);
      let position = Node::<Vec3<f32>>::load_from_u32_buffer(
        &position_buffer,
        mesh_meta.global_position_buffer_offset().load() + vertex_index * val(3),
        StructLayoutTarget::Packed,
      );

      builder.register::<GeometryPosition>(position);

      builder.primitive_state.topology = PrimitiveTopology::TriangleList;
    })
  }
}

impl ShaderPassBuilder for MeshletGPUDraw {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx
      .pass
      .set_index_buffer_by_buffer_resource_view(&self.index_buffer, IndexFormat::Uint32);

    ctx.binding.bind(&self.position_buffer);
    ctx.binding.bind(&self.mesh_src_data);
  }
}
