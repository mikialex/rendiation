use crate::*;

#[derive(Clone)]
pub(super) struct BindlessDrawCreator {
  pub(super) metadata: AbstractReadonlyStorageBuffer<[AttributeMeshMeta]>,
  pub(super) sm_to_mesh: BoxedDynQuery<RawEntityHandle, RawEntityHandle>,
  pub(super) sm_to_mesh_device: AbstractReadonlyStorageBuffer<[u32]>,
  pub(super) vertex_address_buffer_host:
    LockReadGuardHolder<SparseStorageBufferWithHostRaw<AttributeMeshMeta>>,
}
impl NoneIndexedDrawCommandBuilder for BindlessDrawCreator {
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
    Box::new(BindlessDrawCreatorInDevice {
      metadata,
      sm_to_mesh_device,
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.metadata);
    builder.bind(&self.sm_to_mesh_device);
  }
}

impl IndexedDrawCommandBuilder for BindlessDrawCreator {
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
    let end = start + address_info.count;
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
    let node = cx.bind_by(&self.metadata);
    let sm_to_mesh_device = cx.bind_by(&self.sm_to_mesh_device);
    Box::new(BindlessDrawCreatorInDevice {
      metadata: node,
      sm_to_mesh_device,
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.metadata);
    builder.bind(&self.sm_to_mesh_device);
  }
}

impl ShaderHashProvider for BindlessDrawCreator {
  shader_hash_type_id! {}
}

pub struct BindlessDrawCreatorInDevice {
  metadata: ShaderReadonlyPtrOf<[AttributeMeshMeta]>,
  sm_to_mesh_device: ShaderReadonlyPtrOf<[u32]>,
}

impl IndexedDrawCommandBuilderInvocation for BindlessDrawCreatorInDevice {
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndexedIndirectArgsStorage> {
    let mesh_handle: Node<u32> = self.sm_to_mesh_device.index(draw_id).load();
    // shader_assert(mesh_handle.not_equals(val(u32::MAX)));

    let meta = self.metadata.index(mesh_handle).load().expand();
    ENode::<DrawIndexedIndirectArgsStorage> {
      vertex_count: meta.count, // the implementation of range allocate assure the count is zero if allocation failed
      instance_count: val(1),
      base_index: meta.index_offset,
      vertex_offset: val(0),
      base_instance: draw_id,
    }
    .construct()
  }
}

impl NoneIndexedDrawCommandBuilderInvocation for BindlessDrawCreatorInDevice {
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
