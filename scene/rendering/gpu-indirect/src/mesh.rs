use std::{mem::offset_of, ops::Deref, sync::Arc};

use parking_lot::RwLock;
use rendiation_mesh_core::AttributeSemantic;
use rendiation_shader_api::*;
use rendiation_webgpu_midc_downgrade::*;

only_vertex!(IndirectAbstractMeshId, u32);

use crate::*;

pub fn use_bindless_mesh(cx: &mut QueryGPUHookCx) -> Option<MeshGPUBindlessImpl> {
  let init_index_count = 200_000;
  let max_index_count = init_index_count * 100;
  let init_vertex_count = 100_000;
  let max_vertex_count = init_vertex_count * 100;

  let (cx, indices) = cx.use_gpu_init(|gpu| {
    let indices = StorageBufferReadonlyDataView::<[u32]>::create_by_with_extra_usage(
      &gpu.device,
      Some("bindless mesh index pool"),
      ZeroedArrayByArrayLength(init_index_count).into(),
      BufferUsages::INDEX,
    );

    let indices = create_growable_buffer(gpu, indices, max_index_count as u32);
    Arc::new(RwLock::new(GPURangeAllocateMaintainer::new(
      gpu,
      indices,
      max_index_count as u32,
    )))
  });

  let (cx, position) = cx.use_gpu_init(|gpu| {
    Arc::new(RwLock::new(create_storage_buffer_range_allocate_pool(
      gpu,
      "bindless mesh vertex pool: position",
      init_vertex_count,
      max_vertex_count,
    )))
  });
  let (cx, normal) = cx.use_gpu_init(|gpu| {
    Arc::new(RwLock::new(create_storage_buffer_range_allocate_pool(
      gpu,
      "bindless mesh vertex pool: normal",
      init_vertex_count,
      max_vertex_count,
    )))
  });
  let (cx, uv) = cx.use_gpu_init(|gpu| {
    Arc::new(RwLock::new(create_storage_buffer_range_allocate_pool(
      gpu,
      "bindless mesh vertex pool: uv",
      init_vertex_count,
      max_vertex_count,
    )))
  });

  if let GPUQueryHookStage::Inspect(inspector) = &mut cx.stage {
    let buffer_size: u64 = indices.read().gpu().resource.desc.size.into();
    let buffer_size = buffer_size as f32 / 1024.;
    inspector.label(&format!("bindless index, size: {:.2} kb", buffer_size));

    let buffer_size: u64 = position.read().gpu().resource.desc.size.into();
    let buffer_size = buffer_size as f32 / 1024.;
    inspector.label(&format!("bindless position, size: {:.2} kb", buffer_size));

    let buffer_size: u64 = normal.read().gpu().resource.desc.size.into();
    let buffer_size = buffer_size as f32 / 1024.;
    inspector.label(&format!("bindless normal, size: {:.2} kb", buffer_size));

    let buffer_size: u64 = uv.read().gpu().resource.desc.size.into();
    let buffer_size = buffer_size as f32 / 1024.;
    inspector.label(&format!("bindless uv, size: {:.2} kb", buffer_size));
  }

  let attribute_buffer_metadata = use_attribute_buffer_metadata(cx, indices, position, normal, uv);

  let (cx, sm_to_mesh_device) =
    cx.use_storage_buffer::<u32>("scene_model to mesh mapping", 128, u32::MAX);

  let fanout = cx
    .use_dual_query::<StandardModelRefAttributesMeshEntity>()
    .fanout(cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>())
    .use_assure_result(cx);

  fanout
    .clone_except_future()
    .map_raw_handle_or_u32_max_changes()
    .update_storage_array(sm_to_mesh_device, 0);

  let sm_to_mesh = fanout
    .if_resolve_stage()
    .map(|v| v.view().filter_map(|v| v).into_boxed());

  cx.when_render(|| {
    let vertex_address_buffer = attribute_buffer_metadata.read().gpu().clone();
    MeshGPUBindlessImpl {
      indices: indices.clone(),
      position: position.clone(),
      normal: normal.clone(),
      uv: uv.clone(),
      checker: global_entity_component_of::<StandardModelRefAttributesMeshEntity>()
        .read_foreign_key(),
      indices_checker: global_entity_component_of::<SceneBufferViewBufferId<AttributeIndexRef>>()
        .read_foreign_key(),
      topology_checker: global_entity_component_of::<AttributesMeshEntityTopology>().read(),
      vertex_address_buffer,
      vertex_address_buffer_host: attribute_buffer_metadata.make_read_holder(),
      sm_to_mesh_device: sm_to_mesh_device.get_gpu_buffer(),
      sm_to_mesh: sm_to_mesh.unwrap(),
      used_in_midc_downgrade: require_midc_downgrade(&cx.gpu.info),
    }
  })
}

fn use_attribute_indices(
  cx: &mut QueryGPUHookCx,
  index_pool: &UntypedU32Pool,
) -> UseResult<impl DataChanges<Key = RawEntityHandle, Value = [u32; 2]>> {
  let index_buffer_ref = cx.use_dual_query::<SceneBufferViewBufferId<AttributeIndexRef>>();
  let index_buffer_range = cx.use_dual_query::<SceneBufferViewBufferRange<AttributeIndexRef>>();
  let index_item_count = cx.use_dual_query::<SceneBufferViewBufferItemCount<AttributeIndexRef>>();

  let source_info = index_buffer_ref
    .dual_query_union(index_buffer_range, |(a, b)| Some((a?, b?)))
    .dual_query_zip(index_item_count)
    .dual_query_filter_map(|((index, range), count)| index.map(|i| (i, range, count.unwrap())))
    .dual_query_boxed()
    .into_delta_change();

  let (cx, meta_generator) = cx.use_plain_state(|| ReactiveRangeAllocatePool::new(index_pool));

  let gpu = cx.gpu.clone();
  let meta_generator = meta_generator.clone();

  source_info
    .map_only_spawn_stage(move |change| {
      let removed_and_changed_keys = change
        .iter_removed()
        .chain(change.iter_update_or_insert().map(|(k, _)| k));

      let data = get_db_view::<BufferEntityData>();

      enum MaybeConverted<'a> {
        U32(Vec<u32>),
        Original(&'a [u8]),
      }

      let changed_keys = change
        .iter_update_or_insert()
        .map(|(k, (buffer_id, range, count))| {
          let buffer = data.read_ref(buffer_id).unwrap().ptr.deref();

          let buffer = if let Some(range) = range {
            let end = range
              .size
              .map(|v| u64::from(v) as usize)
              .unwrap_or(buffer.len());
            buffer.get(range.offset as usize..end).unwrap()
          } else {
            buffer
          };

          let byte_per_item = buffer.len() / count as usize;
          if byte_per_item != 4 && byte_per_item != 2 {
            unreachable!("index count must be multiple of 2(u16) or 4(u32)")
          }

          let buffer = if byte_per_item == 2 {
            let buffer = bytemuck::cast_slice::<_, u16>(buffer);
            let buffer = buffer.iter().map(|i| *i as u32).collect::<Vec<_>>();
            MaybeConverted::U32(buffer)
          } else {
            MaybeConverted::Original(buffer)
          };

          (k, buffer)
        })
        .collect::<Vec<_>>();

      let changed_keys = changed_keys.iter().map(|(k, v)| match v {
        MaybeConverted::U32(v) => (*k, bytemuck::cast_slice(v)),
        MaybeConverted::Original(v) => (*k, *v),
      });

      meta_generator.update(removed_and_changed_keys, changed_keys, &gpu)
    })
    .into_delta_change()
}

fn use_attribute_vertex(
  cx: &mut QueryGPUHookCx,
  pool: &UntypedU32Pool,
  semantic: AttributeSemantic,
) -> UseResult<impl DataChanges<Key = RawEntityHandle, Value = [u32; 2]>> {
  let attribute_scope = cx
    .use_dual_query::<AttributesMeshEntityVertexBufferSemantic>()
    .dual_query_filter_map(move |s| (semantic == s).then_some(()));

  let (scope, scope_) = attribute_scope.fork();

  let vertex_buffer_ref = cx.use_dual_query::<SceneBufferViewBufferId<AttributeVertexRef>>();
  let vertex_buffer_range = cx.use_dual_query::<SceneBufferViewBufferRange<AttributeVertexRef>>();

  let source_info = vertex_buffer_ref
    .dual_query_union(vertex_buffer_range, |(a, b)| Some((a?, b?)))
    .dual_query_filter_map(|(index, range)| index.map(|i| (i, range)))
    .dual_query_boxed()
    .dual_query_filter_by_set(scope)
    .into_delta_change();

  let (cx, meta_generator) = cx.use_plain_state(|| ReactiveRangeAllocatePool::new(pool));

  let gpu = cx.gpu.clone();
  let meta_generator = meta_generator.clone();

  let allocation_info = source_info.map_only_spawn_stage(move |change| {
    let removed_and_changed_keys = change
      .iter_removed()
      .chain(change.iter_update_or_insert().map(|(k, _)| k));

    let data = get_db_view::<BufferEntityData>();
    let changed_keys = change
      .iter_update_or_insert()
      .map(|(k, (buffer_id, range))| {
        let buffer = data.read_ref(buffer_id).unwrap().ptr.deref();

        let buffer = if let Some(range) = range {
          let end = range
            .size
            .map(|v| u64::from(v) as usize)
            .unwrap_or(buffer.len());
          buffer.get(range.offset as usize..end).unwrap()
        } else {
          buffer
        };

        (k, buffer)
      });

    meta_generator.update(removed_and_changed_keys, changed_keys, &gpu)
  });

  let ab_ref_mesh = cx
    .use_dual_query::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .dual_query_filter_map(|v| v)
    .dual_query_filter_by_set(scope_)
    .use_dual_query_hash_reverse_assume_one_one(cx)
    .dual_query_boxed()
    .use_dual_query_hash_many_to_one(cx);

  allocation_info
    .fanout(ab_ref_mesh)
    .dual_query_boxed()
    .into_delta_change()
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

fn use_attribute_buffer_metadata(
  cx: &mut QueryGPUHookCx,
  index_pool: &UntypedU32Pool,
  position_pool: &UntypedU32Pool,
  normal_pool: &UntypedU32Pool,
  uv_pool: &UntypedU32Pool,
) -> Arc<RwLock<CommonStorageBufferImplWithHostBackup<AttributeMeshMeta>>> {
  let (cx, data) = cx.use_gpu_init(|gpu| {
    let data = create_common_storage_buffer_with_host_backup_container(128, u32::MAX, gpu);
    Arc::new(RwLock::new(data))
  });

  let indices = use_attribute_indices(cx, index_pool);

  let position = use_attribute_vertex(cx, position_pool, AttributeSemantic::Positions);
  let normal = use_attribute_vertex(cx, normal_pool, AttributeSemantic::Normals);
  let uv = use_attribute_vertex(cx, uv_pool, AttributeSemantic::TexCoords(0));

  let offset = offset_of!(AttributeMeshMeta, index_offset);
  use_update(cx, data, indices, offset);
  let offset = offset_of!(AttributeMeshMeta, position_offset);
  use_update(cx, data, position, offset);
  let offset = offset_of!(AttributeMeshMeta, normal_offset);
  use_update(cx, data, normal, offset);
  let offset = offset_of!(AttributeMeshMeta, uv_offset);
  use_update(cx, data, uv, offset);

  fn use_update<T: Pod>(
    cx: &mut QueryGPUHookCx,
    storage: &Arc<RwLock<CommonStorageBufferImplWithHostBackup<AttributeMeshMeta>>>,
    change: UseResult<impl DataChanges<Key = RawEntityHandle, Value = T> + 'static>,
    field_offset: usize,
  ) {
    let change = change.use_assure_result(cx);
    let r = match change {
      UseResult::SpawnStageReady(r) => r,
      UseResult::ResolveStageReady(r) => r,
      _ => return,
    };
    if r.has_change() {
      let mut storage = storage.write();
      for (id, value) in r.iter_update_or_insert() {
        unsafe {
          storage
            .set_value_sub_bytes(id.alloc_index(), field_offset, bytes_of(&value))
            .unwrap();
        }
      }
    }
  }

  data.clone()
}

#[derive(Clone)]
pub struct MeshGPUBindlessImpl {
  indices: UntypedU32Pool,
  position: UntypedU32Pool,
  normal: UntypedU32Pool,
  uv: UntypedU32Pool,
  vertex_address_buffer: StorageBufferReadonlyDataView<[AttributeMeshMeta]>,
  /// we keep the host metadata to support creating draw commands from host
  vertex_address_buffer_host:
    LockReadGuardHolder<CommonStorageBufferImplWithHostBackup<AttributeMeshMeta>>,
  sm_to_mesh_device: StorageBufferReadonlyDataView<[u32]>,
  sm_to_mesh: BoxedDynQuery<RawEntityHandle, RawEntityHandle>,
  checker: ForeignKeyReadView<StandardModelRefAttributesMeshEntity>,
  indices_checker: ForeignKeyReadView<SceneBufferViewBufferId<AttributeIndexRef>>,
  topology_checker: ComponentReadView<AttributesMeshEntityTopology>,
  used_in_midc_downgrade: bool,
}

impl MeshGPUBindlessImpl {
  pub fn make_bindless_dispatcher(&self) -> BindlessMeshDispatcher {
    let position =
      StorageBufferReadonlyDataView::try_from_raw(self.position.read().gpu().gpu.clone()).unwrap();
    let normal =
      StorageBufferReadonlyDataView::try_from_raw(self.normal.read().gpu().gpu.clone()).unwrap();
    let uv = StorageBufferReadonlyDataView::try_from_raw(self.uv.read().gpu().gpu.clone()).unwrap();

    let index_pool =
      StorageBufferReadonlyDataView::try_from_raw(self.indices.read().gpu().gpu.clone()).unwrap();

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
    let mesh = self.checker.get(any_idx)?;
    let is_indexed = self.indices_checker.get(mesh).is_some();
    let topology = self.topology_checker.get(mesh)?;

    let mesh_system = BindlessMeshRasterDispatcher {
      internal: self.make_bindless_dispatcher(),
      topology: map_topology(*topology),
      is_indexed,
    };

    let mesh_system = MidcDowngradeWrapperForIndirectMeshSystem {
      index: is_indexed.then(|| mesh_system.internal.index_pool.clone()),
      mesh_system,
      enable_downgrade: self.used_in_midc_downgrade,
    };

    Some(Box::new(mesh_system))
  }

  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    let mesh_id = self.checker.get(any_id)?;
    let topology = self.topology_checker.get(mesh_id)?;
    topology.hash(hasher);
    let is_index_mesh = self.indices_checker.get(mesh_id).is_some();
    is_index_mesh.hash(hasher);
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

  fn generate_indirect_draw_provider(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    any_idx: EntityHandle<StandardModelEntity>,
    ctx: &mut FrameCtx,
  ) -> Option<Box<dyn IndirectDrawProvider>> {
    let _ = self.checker.get(any_idx)?;

    let draw_command_builder = self.make_draw_command_builder(any_idx).unwrap();

    ctx
      .access_parallel_compute(|cx| {
        batch.create_default_indirect_draw_provider(draw_command_builder, cx)
      })
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

#[derive(Clone)]
pub struct BindlessMeshRasterDispatcher {
  pub internal: BindlessMeshDispatcher,
  pub is_indexed: bool,
  pub topology: PrimitiveTopology,
}

impl ShaderHashProvider for BindlessMeshRasterDispatcher {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.is_indexed.hash(hasher);
    self.topology.hash(hasher);
  }
}

impl ShaderPassBuilder for BindlessMeshRasterDispatcher {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    let mesh = &self.internal;

    if self.is_indexed {
      ctx
        .pass
        .set_index_buffer_by_buffer_resource_view(&mesh.index_pool, IndexFormat::Uint32);
    }

    mesh.bind_base_invocation(&mut ctx.binding);
  }
}

impl GraphicsShaderProvider for BindlessMeshRasterDispatcher {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|vertex, binding| {
      let mesh_handle = vertex.query::<IndirectAbstractMeshId>();

      let vertex_id = vertex.query::<VertexIndex>();

      let mesh_sys = self.internal.build_base_invocation(binding);
      let (position, normal, uv) = mesh_sys.get_position_normal_uv(mesh_handle, vertex_id);

      vertex.register::<GeometryPosition>(position);
      vertex.register::<GeometryNormal>(normal);
      vertex.register::<GeometryUV>(uv);

      vertex.primitive_state.topology = self.topology;
    })
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

    normal_offset.equals(u32::MAX).select_branched(
      || val(Vec3::zero()),
      || {
        let layout = StructLayoutTarget::Packed;
        unsafe {
          Vec3::<f32>::sized_ty()
            .load_from_u32_buffer(&self.normal, normal_offset + vertex_id * val(3), layout)
            .into_node::<Vec3<f32>>()
        }
      },
    )
  }

  pub fn get_position(&self, mesh_handle: Node<u32>, vertex_id: Node<u32>) -> Node<Vec3<f32>> {
    let meta = self.vertex_address_buffer.index(mesh_handle);
    let position_offset = meta.position_offset().load();
    // todo assert position_offset != u32::MAX
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

    uv_offset.equals(u32::MAX).select_branched(
      || val(Vec2::zero()),
      || {
        let layout = StructLayoutTarget::Packed;
        unsafe {
          Vec2::<f32>::sized_ty()
            .load_from_u32_buffer(&self.uv, uv_offset + vertex_id * val(2), layout)
            .into_node::<Vec2<f32>>()
        }
      },
    )
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

#[derive(Clone)]
pub struct BindlessDrawCreator {
  metadata: StorageBufferReadonlyDataView<[AttributeMeshMeta]>,
  sm_to_mesh: BoxedDynQuery<RawEntityHandle, RawEntityHandle>,
  sm_to_mesh_device: StorageBufferReadonlyDataView<[u32]>,
  vertex_address_buffer_host:
    LockReadGuardHolder<CommonStorageBufferImplWithHostBackup<AttributeMeshMeta>>,
}
impl NoneIndexedDrawCommandBuilder for BindlessDrawCreator {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> DrawCommand {
    let mesh_id = self.sm_to_mesh.access(&id.into_raw()).unwrap();
    let address_info = self
      .vertex_address_buffer_host
      .vec
      .get(mesh_id.alloc_index() as usize)
      .unwrap();

    // assert_eq!(address_info.index_offset, u32::MAX); we currently not write u32 for none index mesh

    DrawCommand::Array {
      instances: 0..1,
      vertices: 0..(address_info.position_count / 3),
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
    let mesh_id = self.sm_to_mesh.access(&id.into_raw()).unwrap();
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
  ) -> Node<DrawIndexedIndirectArgsStorage> {
    let mesh_handle: Node<u32> = self.sm_to_mesh_device.index(draw_id).load();
    // shader_assert(mesh_handle.not_equals(val(u32::MAX)));
    // shader_assert(meta.index_offset.not_equals(val(u32::MAX)));

    let meta = self.node.index(mesh_handle).load().expand();
    ENode::<DrawIndexedIndirectArgsStorage> {
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
  ) -> Node<DrawIndirectArgsStorage> {
    let mesh_handle: Node<u32> = self.sm_to_mesh_device.index(draw_id).load();
    // shader_assert(mesh_handle.not_equals(val(u32::MAX)));
    // shader_assert(meta.index_offset.equals(val(u32::MAX)));

    let meta = self.node.index(mesh_handle).load().expand();
    ENode::<DrawIndirectArgsStorage> {
      vertex_count: meta.position_count / val(3),
      instance_count: val(1),
      base_vertex: val(0),
      base_instance: draw_id,
    }
    .construct()
  }
}
