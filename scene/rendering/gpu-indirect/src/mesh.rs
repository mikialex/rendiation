use std::{mem::offset_of, sync::Arc};

use parking_lot::RwLock;
use rendiation_mesh_core::{AttributeSemantic, BufferViewRange};
use rendiation_shader_api::*;

only_vertex!(IndirectAbstractMeshId, u32);

use crate::*;

pub fn use_bindless_mesh(cx: &mut impl QueryGPUHookCx) -> Option<MeshGPUBindlessImpl> {
  let (cx, indices) = cx.use_gpu_init(|gpu| {
    let indices_init_size = 20 * 1024 * 1024;
    let indices_max_size = 200 * 1024 * 1024;

    let indices = StorageBufferReadonlyDataView::<[u32]>::create_by_with_extra_usage(
      &gpu.device,
      ZeroedArrayByArrayLength(indices_init_size as usize).into(),
      BufferUsages::INDEX,
    );

    let indices = create_growable_buffer(gpu, indices, indices_max_size);
    Arc::new(RwLock::new(GPURangeAllocateMaintainer::new(gpu, indices)))
  });

  let (cx, position) = cx.use_gpu_init(|gpu| {
    Arc::new(RwLock::new(create_storage_buffer_range_allocate_pool(
      gpu,
      100 * 1024 * 1024,
      1000 * 1024 * 1024,
    )))
  });
  let (cx, normal) = cx.use_gpu_init(|gpu| {
    Arc::new(RwLock::new(create_storage_buffer_range_allocate_pool(
      gpu,
      100 * 1024 * 1024,
      1000 * 1024 * 1024,
    )))
  });
  let (cx, uv) = cx.use_gpu_init(|gpu| {
    Arc::new(RwLock::new(create_storage_buffer_range_allocate_pool(
      gpu,
      80 * 1024 * 1024,
      1000 * 1024 * 1024,
    )))
  });

  let attribute_buffer_metadata =
    cx.use_multi_updater_gpu(|gpu| attribute_buffer_metadata(gpu, indices, position, normal, uv));

  let sm_to_mesh = cx.when_init(|| {
    global_watch()
      .watch_typed_foreign_key::<StandardModelRefAttributesMeshEntity>()
      .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelStdModelRenderPayload>())
      .into_forker()
  });

  let sm_to_mesh_device = cx.use_storage_buffer(|gpu| {
    let sm_to_mesh_device_source = sm_to_mesh
      .clone()
      .unwrap()
      .collective_map(|v| v.map(|v| v.alloc_index()).unwrap_or(u32::MAX))
      .into_query_update_storage(0);

    create_reactive_storage_buffer_container::<u32>(128, u32::MAX, gpu)
      .with_source(sm_to_mesh_device_source)
  });

  let sm_to_mesh =
    cx.use_reactive_query(|| sm_to_mesh.clone().unwrap().collective_filter_map(|v| v));

  cx.when_render(|| MeshGPUBindlessImpl {
    indices: indices.clone(),
    position: position.clone(),
    normal: normal.clone(),
    uv: uv.clone(),
    checker: global_entity_component_of::<StandardModelRefAttributesMeshEntity>()
      .read_foreign_key(),
    indices_checker: global_entity_component_of::<SceneBufferViewBufferId<AttributeIndexRef>>()
      .read_foreign_key(),
    vertex_address_buffer: attribute_buffer_metadata.clone().unwrap().gpu().clone(),
    vertex_address_buffer_host: attribute_buffer_metadata.unwrap(),
    sm_to_mesh_device: sm_to_mesh_device.unwrap(),
    sm_to_mesh: sm_to_mesh.unwrap(),
  })
}

fn attribute_indices(
  index_pool: &UntypedPool,
  gpu: &GPU,
) -> impl ReactiveQuery<Key = EntityHandle<AttributesMeshEntity>, Value = Vec2<u32>> {
  let index_buffer_ref =
    global_watch().watch_typed_foreign_key::<SceneBufferViewBufferId<AttributeIndexRef>>();
  let index_buffer_range = global_watch().watch::<SceneBufferViewBufferRange<AttributeIndexRef>>();

  // we not using intersect here because range may not exist
  // todo, put it into registry
  let source = index_buffer_ref
    .collective_union(index_buffer_range, |(a, b)| Some((a?, b?)))
    .collective_zip(global_watch().watch::<SceneBufferViewBufferItemCount<AttributeIndexRef>>())
    .collective_filter_map(|((index, range), count)| index.map(|i| (i, range, count.unwrap())))
    .collective_execute_map_by(|| {
      let data = global_entity_component_of::<BufferEntityData>().read();
      move |_, (buffer_id, range, count)| {
        let count = count as usize;
        let buffer = data.get(buffer_id).unwrap().ptr.clone();
        if buffer.len() / count == 4 {
          (buffer, range_convert(range))
        } else if buffer.len() / count == 2 {
          let buffer = bytemuck::cast_slice::<_, u16>(&buffer);
          let buffer = buffer.iter().map(|i| *i as u32).collect::<Vec<_>>();
          let buffer = bytemuck::cast_slice::<_, u8>(buffer.as_slice());
          let buffer = Arc::new(buffer.to_vec());
          (buffer, None)
        } else {
          unreachable!("index count must be 2 or 4")
        }
      }
    })
    .into_boxed();

  ReactiveRangeAllocatePool::new(index_pool, source, gpu)
    .collective_map(|(offset, count)| Vec2::new(offset, count))
}

/// return u32::MAX for all none_indexed mesh
fn none_attribute_mesh_index_indicator(
) -> impl ReactiveQuery<Key = EntityHandle<AttributesMeshEntity>, Value = u32> {
  global_watch()
    .watch_typed_foreign_key::<SceneBufferViewBufferId<AttributeIndexRef>>()
    .collective_filter(|v| v.is_none())
    .collective_map(|_| u32::MAX)
}

fn range_convert(range: Option<BufferViewRange>) -> Option<GPUBufferViewRange> {
  range.map(|r| GPUBufferViewRange {
    offset: r.offset,
    size: r.size,
  })
}

fn attribute_vertex(
  pool: &UntypedPool,
  semantic: AttributeSemantic,
  gpu: &GPU,
) -> impl ReactiveQuery<Key = EntityHandle<AttributesMeshEntity>, Value = [u32; 2]> {
  let attribute_scope = global_watch()
    .watch::<AttributesMeshEntityVertexBufferSemantic>()
    .collective_filter(move |s| semantic == s)
    .collective_map(|_| {})
    .into_forker();

  let vertex_buffer_ref = global_watch()
    .watch_typed_foreign_key::<SceneBufferViewBufferId<AttributeVertexRef>>()
    .filter_by_keyset(attribute_scope.clone());

  let vertex_buffer_range = global_watch()
    .watch::<SceneBufferViewBufferRange<AttributeVertexRef>>()
    .filter_by_keyset(attribute_scope.clone());

  let ranged_buffer = vertex_buffer_ref
    .collective_union(vertex_buffer_range, |(a, b)| Some((a?, b?)))
    .collective_filter_map(|(index, range)| index.map(|i| (i, range)))
    .collective_execute_map_by(|| {
      let data = global_entity_component_of::<BufferEntityData>().read();
      move |_, v| (data.get(v.0).unwrap().ptr.clone(), range_convert(v.1))
    })
    .into_boxed();

  let ab_ref_mesh = global_watch()
    .watch_typed_foreign_key::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .collective_filter_map(|v| v)
    .filter_by_keyset(attribute_scope)
    .hash_reverse_assume_one_one();

  // we not using intersect here because range may not exist
  // todo, put it into registry
  ReactiveRangeAllocatePool::new(pool, ranged_buffer, gpu)
    .collective_map(|v| [v.0, v.1])
    .one_to_many_fanout(ab_ref_mesh.into_one_to_many_by_hash())
}

///  note the attribute's count should be same for one mesh, will keep it here for simplicity
#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, PartialEq, Copy, ShaderStruct, Default)]
pub struct AttributeMeshMeta {
  pub index_offset: u32,
  pub count: u32,
  pub position_offset: u32,
  pub position_count: u32,
  pub normal_offset: u32,
  pub normal_count: u32,
  pub uv_offset: u32,
  pub uv_count: u32,
}

pub fn attribute_buffer_metadata(
  gpu: &GPU,
  index_pool: &UntypedPool,
  position_pool: &UntypedPool,
  normal_pool: &UntypedPool,
  uv_pool: &UntypedPool,
) -> MultiUpdateContainer<CommonStorageBufferImplWithHostBackup<AttributeMeshMeta>> {
  let data = MultiUpdateContainer::new(create_common_storage_buffer_with_host_backup_container(
    128,
    u32::MAX,
    gpu,
  ));

  data
    .with_source(QueryBasedStorageBufferUpdate {
      // note, the offset and count is update together
      field_offset: offset_of!(AttributeMeshMeta, index_offset) as u32,
      upstream: attribute_indices(index_pool, gpu),
    })
    .with_source(QueryBasedStorageBufferUpdate {
      // note, the offset and count is update together
      field_offset: offset_of!(AttributeMeshMeta, index_offset) as u32,
      upstream: none_attribute_mesh_index_indicator(),
    })
    .with_source(QueryBasedStorageBufferUpdate {
      field_offset: offset_of!(AttributeMeshMeta, position_offset) as u32,
      upstream: attribute_vertex(position_pool, AttributeSemantic::Positions, gpu),
    })
    .with_source(QueryBasedStorageBufferUpdate {
      field_offset: offset_of!(AttributeMeshMeta, normal_offset) as u32,
      upstream: attribute_vertex(normal_pool, AttributeSemantic::Normals, gpu),
    })
    .with_source(QueryBasedStorageBufferUpdate {
      field_offset: offset_of!(AttributeMeshMeta, uv_offset) as u32,
      upstream: attribute_vertex(uv_pool, AttributeSemantic::TexCoords(0), gpu),
    })
}

pub struct MeshGPUBindlessImpl {
  indices: UntypedPool,
  position: UntypedPool,
  normal: UntypedPool,
  uv: UntypedPool,
  vertex_address_buffer: StorageBufferReadonlyDataView<[AttributeMeshMeta]>,
  vertex_address_buffer_host: LockReadGuardHolder<
    MultiUpdateContainer<CommonStorageBufferImplWithHostBackup<AttributeMeshMeta>>,
  >,
  sm_to_mesh_device: StorageBufferReadonlyDataView<[u32]>,
  sm_to_mesh: BoxedDynQuery<EntityHandle<SceneModelEntity>, EntityHandle<AttributesMeshEntity>>,
  checker: ForeignKeyReadView<StandardModelRefAttributesMeshEntity>,
  indices_checker: ForeignKeyReadView<SceneBufferViewBufferId<AttributeIndexRef>>,
}

impl MeshGPUBindlessImpl {
  pub fn make_bindless_dispatcher(&self) -> BindlessMeshDispatcher {
    let position =
      StorageBufferReadonlyDataView::try_from_raw(self.position.read().raw_gpu().clone()).unwrap();
    let normal =
      StorageBufferReadonlyDataView::try_from_raw(self.normal.read().raw_gpu().clone()).unwrap();
    let uv = StorageBufferReadonlyDataView::try_from_raw(self.uv.read().raw_gpu().clone()).unwrap();

    let index_pool =
      StorageBufferReadonlyDataView::try_from_raw(self.indices.read().raw_gpu().clone()).unwrap();

    BindlessMeshDispatcher {
      sm_to_mesh: self.sm_to_mesh_device.clone(),
      vertex_address_buffer: self.vertex_address_buffer.clone(),
      position,
      normal,
      uv,
      index_pool,
    }
  }
}

impl IndirectModelShapeRenderImpl for MeshGPUBindlessImpl {
  fn make_component_indirect(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    // check the given model has attributes mesh
    let mesh_id = self.checker.get(any_idx)?;
    // check mesh must have indices.
    let _ = self.indices_checker.get(mesh_id)?;
    Some(Box::new(self.make_bindless_dispatcher()))
  }

  fn hash_shader_group_key(
    &self,
    _any_id: EntityHandle<StandardModelEntity>,
    _hasher: &mut PipelineHasher,
  ) -> Option<()> {
    Some(())
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<DrawCommandBuilder> {
    // check the given model has attributes mesh
    let mesh_id = self.checker.get(any_idx)?;
    // check mesh must have indices.
    let is_indexed = self.indices_checker.get(mesh_id).is_some();

    let creator = BindlessDrawCreator {
      metadata: self.vertex_address_buffer.clone(),
      sm_to_mesh_device: self.sm_to_mesh_device.clone(),
      sm_to_mesh: self.sm_to_mesh.clone(),
      vertex_address_buffer_host: self.vertex_address_buffer_host.clone(),
    };

    if is_indexed {
      DrawCommandBuilder::Indexed(Box::new(creator))
    } else {
      DrawCommandBuilder::NoneIndexed(Box::new(creator))
    }
    .into()
  }
}

#[derive(Clone)]
pub struct BindlessMeshDispatcher {
  pub sm_to_mesh: StorageBufferReadonlyDataView<[u32]>,
  pub vertex_address_buffer: StorageBufferReadonlyDataView<[AttributeMeshMeta]>,
  pub index_pool: StorageBufferReadonlyDataView<[u32]>,
  pub position: StorageBufferReadonlyDataView<[u32]>,
  pub normal: StorageBufferReadonlyDataView<[u32]>,
  pub uv: StorageBufferReadonlyDataView<[u32]>,
}

impl ShaderHashProvider for BindlessMeshDispatcher {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for BindlessMeshDispatcher {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx
      .pass
      .set_index_buffer_by_buffer_resource_view(&self.index_pool, IndexFormat::Uint32);

    self.bind_base_invocation(&mut ctx.binding);
  }
}

#[derive(Clone)]
pub struct BindlessMeshDispatcherBaseInvocation {
  pub vertex_address_buffer: ShaderReadonlyPtrOf<[AttributeMeshMeta]>,
  pub position: ShaderReadonlyPtrOf<[u32]>,
  pub normal: ShaderReadonlyPtrOf<[u32]>,
  pub uv: ShaderReadonlyPtrOf<[u32]>,
}

impl BindlessMeshDispatcherBaseInvocation {
  pub fn get_position_normal_uv(
    &self,
    mesh_handle: Node<u32>,
    vertex_id: Node<u32>,
  ) -> (Node<Vec3<f32>>, Node<Vec3<f32>>, Node<Vec2<f32>>) {
    let position = self.get_position(mesh_handle, vertex_id);
    let normal = self.get_normal(mesh_handle, vertex_id);
    let uv = self.get_uv(mesh_handle, vertex_id);
    (position, normal, uv)
  }

  pub fn get_normal(&self, mesh_handle: Node<u32>, vertex_id: Node<u32>) -> Node<Vec3<f32>> {
    let meta = self.vertex_address_buffer.index(mesh_handle);
    let normal_offset = meta.normal_offset().load();
    let layout = StructLayoutTarget::Packed;
    unsafe {
      Vec3::<f32>::sized_ty()
        .load_from_u32_buffer(&self.normal, normal_offset + vertex_id * val(3), layout)
        .into_node::<Vec3<f32>>()
    }
  }

  pub fn get_position(&self, mesh_handle: Node<u32>, vertex_id: Node<u32>) -> Node<Vec3<f32>> {
    let meta = self.vertex_address_buffer.index(mesh_handle);
    let position_offset = meta.position_offset().load();
    let layout = StructLayoutTarget::Packed;
    unsafe {
      Vec3::<f32>::sized_ty()
        .load_from_u32_buffer(&self.position, position_offset + vertex_id * val(3), layout)
        .into_node::<Vec3<f32>>()
    }
  }

  pub fn get_uv(&self, mesh_handle: Node<u32>, vertex_id: Node<u32>) -> Node<Vec2<f32>> {
    let meta = self.vertex_address_buffer.index(mesh_handle);
    let uv_offset = meta.uv_offset().load();
    let layout = StructLayoutTarget::Packed;
    unsafe {
      Vec2::<f32>::sized_ty()
        .load_from_u32_buffer(&self.uv, uv_offset + vertex_id * val(2), layout)
        .into_node::<Vec2<f32>>()
    }
  }
}

impl BindlessMeshDispatcher {
  pub fn build_base_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> BindlessMeshDispatcherBaseInvocation {
    BindlessMeshDispatcherBaseInvocation {
      vertex_address_buffer: cx.bind_by(&self.vertex_address_buffer),
      position: cx.bind_by(&self.position),
      normal: cx.bind_by(&self.position),
      uv: cx.bind_by(&self.position),
    }
  }
  pub fn bind_base_invocation(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.vertex_address_buffer);
    cx.bind(&self.position);
    cx.bind(&self.normal);
    cx.bind(&self.uv);
  }
}

impl GraphicsShaderProvider for BindlessMeshDispatcher {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|vertex, binding| {
      let mesh_handle = vertex.query::<IndirectAbstractMeshId>();
      let vertex_id = vertex.query::<VertexIndex>();

      let mesh_sys = self.build_base_invocation(binding);
      let (position, normal, uv) = mesh_sys.get_position_normal_uv(mesh_handle, vertex_id);

      vertex.register::<GeometryPosition>(position);
      vertex.register::<GeometryNormal>(normal);
      vertex.register::<GeometryUV>(uv);
    })
  }
}

#[derive(Clone)]
pub struct BindlessDrawCreator {
  metadata: StorageBufferReadonlyDataView<[AttributeMeshMeta]>,
  sm_to_mesh: BoxedDynQuery<EntityHandle<SceneModelEntity>, EntityHandle<AttributesMeshEntity>>,
  sm_to_mesh_device: StorageBufferReadonlyDataView<[u32]>,
  vertex_address_buffer_host: LockReadGuardHolder<
    MultiUpdateContainer<CommonStorageBufferImplWithHostBackup<AttributeMeshMeta>>,
  >,
}
impl NoneIndexedDrawCommandBuilder for BindlessDrawCreator {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> DrawCommand {
    let mesh_id = self.sm_to_mesh.access(&id).unwrap();
    let address_info = self
      .vertex_address_buffer_host
      .vec
      .get(mesh_id.alloc_index() as usize)
      .unwrap();

    let start = address_info.position_offset;
    let end = start + address_info.position_count / 3;
    DrawCommand::Array {
      instances: 0..1,
      vertices: start..end,
    }
  }

  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn NoneIndexedDrawCommandBuilderInvocation> {
    let node = cx.bind_by(&self.metadata);
    let sm_to_mesh_device = cx.bind_by(&self.sm_to_mesh_device);
    Box::new(BindlessDrawCreatorInDevice {
      node,
      sm_to_mesh_device,
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.metadata);
    builder.bind(&self.sm_to_mesh_device);
  }
}

impl IndexedDrawCommandBuilder for BindlessDrawCreator {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> DrawCommand {
    let mesh_id = self.sm_to_mesh.access(&id).unwrap();
    let address_info = self
      .vertex_address_buffer_host
      .vec
      .get(mesh_id.alloc_index() as usize)
      .unwrap();

    let start = address_info.index_offset;
    let end = start + address_info.count;
    DrawCommand::Indexed {
      base_vertex: 0,
      indices: start..end,
      instances: 0..1,
    }
  }

  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn IndexedDrawCommandBuilderInvocation> {
    let node = cx.bind_by(&self.metadata);
    let sm_to_mesh_device = cx.bind_by(&self.sm_to_mesh_device);
    Box::new(BindlessDrawCreatorInDevice {
      node,
      sm_to_mesh_device,
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.metadata);
    builder.bind(&self.sm_to_mesh_device);
  }
}

impl ShaderPassBuilder for BindlessDrawCreator {
  fn setup_pass(&self, cx: &mut GPURenderPassCtx) {
    cx.binding.bind(&self.metadata);
  }
}

impl ShaderHashProvider for BindlessDrawCreator {
  shader_hash_type_id! {}
}

pub struct BindlessDrawCreatorInDevice {
  node: ShaderReadonlyPtrOf<[AttributeMeshMeta]>,
  sm_to_mesh_device: ShaderReadonlyPtrOf<[u32]>,
}

impl IndexedDrawCommandBuilderInvocation for BindlessDrawCreatorInDevice {
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndexedIndirect> {
    let mesh_handle: Node<u32> = self.sm_to_mesh_device.index(draw_id).load();
    // shader_assert(mesh_handle.not_equals(val(u32::MAX)));

    let meta = self.node.index(mesh_handle).load().expand();
    ENode::<DrawIndexedIndirect> {
      vertex_count: meta.count,
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
  ) -> Node<DrawIndirect> {
    let mesh_handle: Node<u32> = self.sm_to_mesh_device.index(draw_id).load();
    // shader_assert(mesh_handle.not_equals(val(u32::MAX)));

    let meta = self.node.index(mesh_handle).load().expand();
    ENode::<DrawIndirect> {
      vertex_count: meta.position_count / val(3),
      instance_count: val(1),
      first_vertex: meta.position_offset,
      first_instance: draw_id,
    }
    .construct()
  }
}
