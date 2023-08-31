use crate::*;

impl GPUBindlessMeshSystem {
  pub fn create_host_draw_dispatcher(
    &self,
    iter: impl Iterator<Item = MeshSystemMeshHandle> + 'static,
    device: &GPUDevice,
  ) -> BindlessMeshDispatcher {
    let (draw, draw_info): (Vec<_>, Vec<_>) = self.map_draw_command_buffer_in_host(iter).unzip();
    let draw_indirect_buffer = GPUBuffer::create(
      device,
      BufferInit::WithInit(bytemuck::cast_slice(&draw)),
      BufferUsages::INDIRECT,
    );
    let vertex_address_buffer =
      StorageBufferReadOnlyDataView::create(device, bytemuck::cast_slice(&draw_info));
    BindlessMeshDispatcher {
      draw_indirect_buffer,
      vertex_address_buffer,
      system: self,
    }
  }

  pub fn map_draw_command_buffer_in_host(
    &self,
    iter: impl Iterator<Item = MeshSystemMeshHandle> + 'static,
  ) -> impl Iterator<Item = (DrawIndirect, DrawVertexIndirectInfo)> + '_ {
    iter.enumerate().map(|(i, handle)| {
        let DrawMetaData { start,  count, vertex_info, .. } = self.meta_data.get(handle as usize).unwrap();
        let draw_indirect = DrawIndirect {
          vertex_count: *count,
          instance_count: 1,
          base_vertex: *start,
          base_instance: i as u32, // we rely on this to get draw id. https://www.g-truc.net/post-0518.html
        };
        (draw_indirect, *vertex_info)
      })
  }
}

pub struct BindlessMeshDispatcher<'a> {
  draw_indirect_buffer: GPUBuffer,
  vertex_address_buffer: StorageBufferReadOnlyDataView<[DrawVertexIndirectInfo]>,
  system: &'a GPUBindlessMeshSystem,
}

impl<'a> BindlessMeshDispatcher<'a> {
  pub fn draw_command(&self) -> DrawCommand {
    DrawCommand::Indirect {
      buffer: self.draw_indirect_buffer.clone(),
    }
  }
}

impl<'a> ShaderHashProvider for BindlessMeshDispatcher<'a> {}

impl<'a> ShaderPassBuilder for BindlessMeshDispatcher<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.vertex_address_buffer);

    let sys = self.system;
    ctx.binding.bind(&sys.bindless_position_vertex_buffers);
    ctx.binding.bind(&sys.bindless_normal_vertex_buffers);
    ctx.binding.bind(&sys.bindless_uv_vertex_buffers);
  }
}

impl<'a> GraphicsShaderProvider for BindlessMeshDispatcher<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|vertex, binding| {
      let draw_id = vertex.query::<VertexInstanceIndex>().unwrap();
      let vertex_id = vertex.query::<VertexIndex>().unwrap();

      let vertex_addresses = binding.bind_by(&self.vertex_address_buffer);
      let vertex_address = vertex_addresses.index(draw_id).load().expand();

      let position = binding.bind_by(&self.system.bindless_position_vertex_buffers);
      let position = position.index(vertex_address.position_buffer_id);
      let position = position
        .index(vertex_address.position_buffer_offset + vertex_id)
        .load();

      let normal = binding.bind_by(&self.system.bindless_normal_vertex_buffers);
      let normal = normal.index(vertex_address.position_buffer_id);
      let normal = normal
        .index(vertex_address.normal_buffer_offset + vertex_id)
        .load();

      let uv = binding.bind_by(&self.system.bindless_uv_vertex_buffers);
      let uv = uv.index(vertex_address.position_buffer_id);
      let uv = uv.index(vertex_address.uv_buffer_offset + vertex_id).load();

      vertex.register::<GeometryPosition>(position);
      vertex.register::<GeometryNormal>(normal);
      vertex.register::<GeometryUV>(uv);
      Ok(())
    })
  }
}
