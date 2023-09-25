use crate::*;

impl GPUBindlessMeshSystem {
  pub fn create_host_draw_dispatcher(
    &self,
    iter: impl Iterator<Item = MeshSystemMeshHandle>,
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
      system: self.clone(),
    }
  }

  pub fn map_draw_command_buffer_in_host<'a>(
    &'a self,
    iter: impl Iterator<Item = MeshSystemMeshHandle> + 'a,
  ) -> impl Iterator<Item = (DrawIndexedIndirect, DrawVertexIndirectInfo)> + 'a {
    dbg!("list:");
    iter.enumerate().map(|(i, handle)| {
      let sys = self.inner.read().unwrap();
        let DrawMetaData { start,  count, vertex_info, .. } = sys.metadata.get(handle as usize).unwrap();
        let draw_indirect = DrawIndexedIndirect {
          vertex_count: *count,
          instance_count: 1,
          base_index: *start,
          vertex_offset: 0,
          base_instance: i as u32, // we rely on this to get draw id. https://www.g-truc.net/post-0518.html
        };
        dbg!(draw_indirect);
        dbg!(vertex_info);
        (draw_indirect, *vertex_info)
      })
  }
}

pub struct BindlessMeshDispatcher {
  draw_indirect_buffer: GPUBuffer,
  vertex_address_buffer: StorageBufferReadOnlyDataView<[DrawVertexIndirectInfo]>,
  system: GPUBindlessMeshSystem,
}

impl BindlessMeshDispatcher {
  pub fn draw_command(&self) -> DrawCommand {
    let size: u64 = self.draw_indirect_buffer.size().into();
    DrawCommand::MultiIndirect {
      indirect_buffer: self.draw_indirect_buffer.clone(),
      indexed: true,
      indirect_offset: 0,
      count: size as u32 / 20,
    }
  }
}

impl ShaderHashProvider for BindlessMeshDispatcher {}

impl ShaderPassBuilder for BindlessMeshDispatcher {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.vertex_address_buffer);

    let sys = self.system.inner.read().unwrap();
    let index = sys.index_buffer.get_buffer();
    ctx.pass.set_index_buffer_owned(&index, IndexFormat::Uint32);

    ctx.binding.bind(&sys.bindless_position_vertex_buffers);
    ctx.binding.bind(&sys.bindless_normal_vertex_buffers);
    ctx.binding.bind(&sys.bindless_uv_vertex_buffers);
  }
}

impl GraphicsShaderProvider for BindlessMeshDispatcher {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.log_result = true;
    builder.vertex(|vertex, binding| {
      let draw_id = vertex.query::<VertexInstanceIndex>().unwrap();
      let vertex_id = vertex.query::<VertexIndex>().unwrap();

      let vertex_addresses = binding.bind_by(&self.vertex_address_buffer);
      // let vertex_address = vertex_addresses.index(draw_id).load().expand();

      let sys = self.system.inner.read().unwrap();

      let position = binding.bind_by(&sys.bindless_position_vertex_buffers);
      // let position = position.index(vertex_address.position_buffer_id);
      // let position = BindlessStorageWorkaround::read_index_shader(position, vertex_id).load();

      let normal = binding.bind_by(&sys.bindless_normal_vertex_buffers);
      // let normal = normal.index(vertex_address.normal_buffer_id);
      // let normal = BindlessStorageWorkaround::read_index_shader(normal, vertex_id).load();

      let uv = binding.bind_by(&sys.bindless_uv_vertex_buffers);
      // let uv = uv.index(vertex_address.uv_buffer_id);
      // let uv = BindlessStorageWorkaround::read_index_shader(uv, vertex_id).load();

      // vertex.register::<GeometryPosition>(position.xyz());
      // vertex.register::<GeometryNormal>(normal.xyz());
      // vertex.register::<GeometryUV>(uv);
      vertex.register::<GeometryPosition>(val(Vec3::zero()));
      vertex.register::<GeometryNormal>(val(Vec3::zero()));
      vertex.register::<GeometryUV>(val(Vec2::zero()));
      Ok(())
    })
  }
}
