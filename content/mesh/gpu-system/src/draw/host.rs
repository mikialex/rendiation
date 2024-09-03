use crate::*;

impl GPUBindlessMeshSystem {
  pub fn create_host_draw_dispatcher(
    &self,
    iter: impl Iterator<Item = MeshSystemMeshHandle>,
    device: &GPUDevice,
  ) -> BindlessMeshDispatcher {
    let (draw, draw_info): (Vec<_>, Vec<_>) = self.map_draw_command_buffer_in_host(iter).unzip();
    let draw_indirect_buffer =
      create_gpu_buffer(bytemuck::cast_slice(&draw), BufferUsages::INDIRECT, device)
        .create_default_view();
    let vertex_address_buffer =
      StorageBufferReadOnlyDataView::create(device, bytemuck::cast_slice(&draw_info));
    BindlessMeshDispatcher {
      draw_indirect_buffer,
      vertex_address_buffer,
      system: self,
    }
  }

  pub fn map_draw_command_buffer_in_host<'a>(
    &'a self,
    iter: impl Iterator<Item = MeshSystemMeshHandle> + 'a,
  ) -> impl Iterator<Item = (DrawIndexedIndirect, DrawVertexIndirectInfo)> + 'a {
    iter.enumerate().map(|(i, handle)| {
        let DrawMetaData { start,  count, vertex_info, .. } = self.metadata.get(handle as usize).unwrap();
        let draw_indirect = DrawIndexedIndirect {
          vertex_count: *count,
          instance_count: 1,
          base_index: *start,
          vertex_offset: 0,
          base_instance: i as u32, // we rely on this to get draw id. https://www.g-truc.net/post-0518.html
        };
        (draw_indirect, *vertex_info)
      })
  }
}

pub struct BindlessMeshDispatcher<'a> {
  draw_indirect_buffer: GPUBufferResourceView,
  vertex_address_buffer: StorageBufferReadOnlyDataView<[DrawVertexIndirectInfo]>,
  system: &'a GPUBindlessMeshSystem,
}

impl<'a> BindlessMeshDispatcher<'a> {
  pub fn draw_command(&self) -> DrawCommand {
    let size: u64 = self.draw_indirect_buffer.view_byte_size().into();
    DrawCommand::MultiIndirect {
      indirect_buffer: self.draw_indirect_buffer.clone(),
      indexed: true,
      indirect_offset: 0,
      count: size as u32 / 20,
    }
  }
}

impl<'a> ShaderHashProvider for BindlessMeshDispatcher<'a> {
  shader_hash_type_id! { BindlessMeshDispatcher<'static> }
}

impl<'a> ShaderPassBuilder for BindlessMeshDispatcher<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.vertex_address_buffer);

    let index = self.system.index_buffer.buffer.buffer();
    ctx
      .pass
      .set_index_buffer_by_buffer_resource_view(index, IndexFormat::Uint32);

    ctx.binding.bind(&self.system.position);
    ctx.binding.bind(&self.system.normal);
    ctx.binding.bind(&self.system.uv);
  }
}

impl<'a> GraphicsShaderProvider for BindlessMeshDispatcher<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.log_result = true;
    builder.vertex(|vertex, binding| {
      let draw_id = vertex.query::<VertexInstanceIndex>().unwrap();
      let vertex_id = vertex.query::<VertexIndex>().unwrap();

      let vertex_addresses = binding.bind_by(&self.vertex_address_buffer);
      let vertex_address = vertex_addresses.index(draw_id).load().expand();

      let position = binding.bind_by(&self.system.position);
      let position = position
        .index(vertex_address.position_buffer_offset + vertex_id)
        .load();

      let normal = binding.bind_by(&self.system.normal);
      let normal = normal
        .index(vertex_address.normal_buffer_offset + vertex_id)
        .load();

      let uv = binding.bind_by(&self.system.uv);
      let uv = uv.index(vertex_address.uv_buffer_offset + vertex_id).load();

      vertex.register::<GeometryPosition>(position.xyz());
      vertex.register::<GeometryNormal>(normal.xyz());
      vertex.register::<GeometryUV>(uv.xy());
      Ok(())
    })
  }
}
