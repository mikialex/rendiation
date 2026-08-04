use crate::*;

#[derive(Clone)]
pub struct AttributeMeshIndirectDrawCreator {
  pub metadata: AbstractReadonlyStorageBuffer<[AttributeMeshMeta]>,
  pub sm_to_mesh: BoxedDynQuery<RawEntityHandle, RawEntityHandle>,
  pub sm_to_mesh_device: AbstractReadonlyStorageBuffer<[u32]>,
  pub vertex_address_buffer_host:
    LockReadGuardHolder<SparseStorageBufferWithHostRaw<AttributeMeshMeta>>,
  pub used_in_midc_downgrade: bool,
}
impl NoneIndexedDrawCommandBuilder for AttributeMeshIndirectDrawCreator {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> Option<DrawCommand> {
    let mesh_id = self.sm_to_mesh.access(&id.into_raw()).unwrap();
    let address_info = self
      .vertex_address_buffer_host
      .get(mesh_id.alloc_index())
      .unwrap();

    if address_info.position_count == DEVICE_RANGE_ALLOCATE_FAIL_MARKER {
      return None;
    }

    DrawCommand::Array {
      instances: 0..1,
      vertices: 0..(address_info.position_count / 3),
    }
    .into()
  }

  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn NoneIndexedDrawCommandBuilderInvocation> {
    let metadata = cx.bind_by(&self.metadata);
    let sm_to_mesh_device = cx.bind_by(&self.sm_to_mesh_device);
    Box::new(AttributeMeshIndirectDrawCreatorInvocation {
      metadata,
      sm_to_mesh_device,
      used_in_midc_downgrade: self.used_in_midc_downgrade,
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.metadata);
    builder.bind(&self.sm_to_mesh_device);
  }
}

impl IndexedDrawCommandBuilder for AttributeMeshIndirectDrawCreator {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> Option<DrawCommand> {
    let mesh_id = self.sm_to_mesh.access(&id.into_raw()).unwrap();
    let address_info = self
      .vertex_address_buffer_host
      .get(mesh_id.alloc_index())
      .unwrap();

    if address_info.index_offset == DEVICE_RANGE_ALLOCATE_FAIL_MARKER {
      return None;
    }

    let start = address_info.index_offset;
    // the host driven path expands the vertex stream per index, u16 count is in u16 units
    let count = if address_info.is_u16_indices.into() {
      if address_info.is_u16_indices_padded.into() {
        address_info.count * 2 - 1
      } else {
        address_info.count * 2
      }
    } else {
      address_info.count
    };
    let end = start + count;
    DrawCommand::Indexed {
      base_vertex: 0,
      indices: start..end,
      instances: 0..1,
    }
    .into()
  }

  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn IndexedDrawCommandBuilderInvocation> {
    let metadata = cx.bind_by(&self.metadata);
    let sm_to_mesh_device = cx.bind_by(&self.sm_to_mesh_device);
    Box::new(AttributeMeshIndirectDrawCreatorInvocation {
      metadata,
      sm_to_mesh_device,
      used_in_midc_downgrade: self.used_in_midc_downgrade,
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.metadata);
    builder.bind(&self.sm_to_mesh_device);
  }
}

impl ShaderHashProvider for AttributeMeshIndirectDrawCreator {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.used_in_midc_downgrade);
  }
}

pub struct AttributeMeshIndirectDrawCreatorInvocation {
  metadata: ShaderReadonlyPtrOf<[AttributeMeshMeta]>,
  sm_to_mesh_device: ShaderReadonlyPtrOf<[u32]>,
  used_in_midc_downgrade: bool,
}

impl IndexedDrawCommandBuilderInvocation for AttributeMeshIndirectDrawCreatorInvocation {
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndexedIndirectArgsStorage> {
    let mesh_handle: Node<u32> = self.sm_to_mesh_device.index(draw_id).load();
    // shader_assert(mesh_handle.not_equals(val(u32::MAX)));

    let meta = self.metadata.index(mesh_handle);

    // the implementation of range allocate assure the count is zero if allocation failed
    let vertex_count = meta.count().load();
    let base_index = meta.index_offset().load();

    let is_u16 = meta.is_u16_indices().load().into_bool();
    let is_u16_padded = meta.is_u16_indices_padded().load().into_bool();

    // u16 indices are packed two per u32, an odd count leaves the last u32 holding only
    // one index plus 2 padding bytes, so the real index count is 2 * count - 1
    let vertex_count = is_u16.select(vertex_count * val(2), vertex_count);
    let vertex_count = is_u16_padded.select(vertex_count - val(1), vertex_count);

    // in midc downgrade mode the pool is read on device with u32 indices, in native midc
    // draw the index buffer is bound as u16, so the first index is in u16 units, so it should be multiplied by 2
    let base_index = if self.used_in_midc_downgrade {
      base_index
    } else {
      is_u16.select(base_index * val(2), base_index)
    };

    ENode::<DrawIndexedIndirectArgsStorage> {
      vertex_count,
      instance_count: val(1),
      base_index,
      vertex_offset: val(0),
      base_instance: draw_id,
    }
    .construct()
  }
}

impl NoneIndexedDrawCommandBuilderInvocation for AttributeMeshIndirectDrawCreatorInvocation {
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndirectArgsStorage> {
    let mesh_handle: Node<u32> = self.sm_to_mesh_device.index(draw_id).load();
    // shader_assert(mesh_handle.not_equals(val(u32::MAX)));

    let meta = self.metadata.index(mesh_handle).load().expand();
    ENode::<DrawIndirectArgsStorage> {
      vertex_count: meta.position_count / val(3), // the implementation of range allocate assure the count is zero if allocation failed
      instance_count: val(1),
      base_vertex: val(0),
      base_instance: draw_id,
    }
    .construct()
  }
}
