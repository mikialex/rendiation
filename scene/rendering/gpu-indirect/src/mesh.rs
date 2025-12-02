use std::{mem::offset_of, sync::Arc};

use parking_lot::RwLock;
use rendiation_mesh_core::{AttributeReadSchema, AttributeSemantic};
use rendiation_shader_api::*;
use rendiation_webgpu_midc_downgrade::*;

only_vertex!(IndirectAbstractMeshId, u32);

use crate::*;

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct BindlessMeshInit {
  pub init_index_count: u32,
  pub max_index_count: u32,
  pub init_vertex_count: u32,
  pub max_vertex_count: u32,
}

impl Default for BindlessMeshInit {
  fn default() -> Self {
    Self {
      init_index_count: 200_000,
      max_index_count: 200_000 * 100,
      init_vertex_count: 100_000,
      max_vertex_count: 100_000 * 100,
    }
  }
}

pub fn use_bindless_mesh(
  cx: &mut QueryGPUHookCx,
  init: &BindlessMeshInit,
  merge_with_vertex_allocator: bool,
  use_midc_downgrade: bool,
) -> Option<MeshGPUBindlessImpl> {
  let force_midc_downgrade = use_midc_downgrade || merge_with_vertex_allocator;

  let BindlessMeshInit {
    init_index_count,
    max_index_count,
    init_vertex_count,
    max_vertex_count,
  } = *init;

  let (indices_range_change, indices) = use_attribute_indices_updates(
    cx,
    max_index_count,
    init_index_count,
    merge_with_vertex_allocator,
  );

  let max = max_vertex_count;
  let init = init_vertex_count;
  let (position_range_change, position) =
    use_attribute_vertex_updates(cx, max, init, AttributeSemantic::Positions);
  let (normal_range_change, normal) =
    use_attribute_vertex_updates(cx, max, init, AttributeSemantic::Normals);
  let (uv_range_change, uv) =
    use_attribute_vertex_updates(cx, max, init, AttributeSemantic::TexCoords(0));

  let (cx, metadata) = cx.use_storage_buffer_with_host_backup::<AttributeMeshMeta>(
    "mesh buffer indirect range",
    128,
    u32::MAX,
  );

  let offset = offset_of!(AttributeMeshMeta, index_offset);
  indices_range_change.update_storage_array_with_host(cx, metadata, offset);

  let offset = offset_of!(AttributeMeshMeta, position_offset);
  position_range_change.update_storage_array_with_host(cx, metadata, offset);

  let offset = offset_of!(AttributeMeshMeta, normal_offset);
  normal_range_change.update_storage_array_with_host(cx, metadata, offset);

  let offset = offset_of!(AttributeMeshMeta, uv_offset);
  uv_range_change.update_storage_array_with_host(cx, metadata, offset);

  metadata.use_max_item_count_by_db_entity::<AttributesMeshEntity>(cx);
  metadata.use_update(cx);

  let (cx, sm_to_mesh_device) =
    cx.use_storage_buffer::<u32>("scene_model to mesh mapping", 128, u32::MAX);

  let relation = cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>();
  let (fanout, fanout_) = cx
    .use_dual_query::<StandardModelRefAttributesMeshEntity>()
    .fanout(relation, cx)
    .fork();

  fanout
    .map_raw_handle_or_u32_max_changes()
    .update_storage_array(cx, sm_to_mesh_device, 0);

  sm_to_mesh_device.use_max_item_count_by_db_entity::<SceneModelEntity>(cx);
  sm_to_mesh_device.use_update(cx);

  let sm_to_mesh = fanout_
    .map(|v| v.view().filter_map(|v| v).into_boxed())
    .use_assure_result(cx);

  cx.when_render(|| {
    let vertex_address_buffer = metadata.get_gpu_buffer();
    MeshGPUBindlessImpl {
      indices,
      position,
      normal,
      uv,
      checker: global_entity_component_of::<StandardModelRefAttributesMeshEntity>()
        .read_foreign_key(),
      indices_checker: global_entity_component_of::<SceneBufferViewBufferId<AttributeIndexRef>>()
        .read_foreign_key(),
      topology_checker: global_entity_component_of::<AttributesMeshEntityTopology>().read(),
      vertex_address_buffer,
      vertex_address_buffer_host: metadata.buffer.make_read_holder(),
      sm_to_mesh_device: sm_to_mesh_device.get_gpu_buffer(),
      sm_to_mesh: sm_to_mesh.expect_resolve_stage(),
      used_in_midc_downgrade: require_midc_downgrade(&cx.gpu.info, force_midc_downgrade),
    }
  })
}

fn use_attribute_indices_updates(
  cx: &mut QueryGPUHookCx,
  max_item_count: u32,
  init_item_count: u32,
  merge_with_vertex_allocator: bool,
) -> (
  UseResult<impl DataChanges<Key = RawEntityHandle, Value = [u32; 2]> + 'static>,
  AbstractReadonlyStorageBuffer<[u32]>,
) {
  let (cx, gpu_buffer) = cx.use_gpu_init(|gpu, alloc| {
    let indices = if merge_with_vertex_allocator {
      alloc.allocate_readonly::<[u32]>(
        (4 * init_item_count) as u64,
        &gpu.device,
        Some("bindless mesh index pool"),
      )
    } else {
      StorageBufferReadonlyDataView::<[u32]>::create_by_with_extra_usage(
        &gpu.device,
        Some("bindless mesh index pool"),
        ZeroedArrayByArrayLength(init_item_count as usize).into(),
        BufferUsages::INDEX,
      )
      .into()
    };

    let indices = indices.with_direct_resize(gpu);

    Arc::new(RwLock::new(indices))
  });

  if let GPUQueryHookStage::Inspect(inspector) = &mut cx.stage {
    let buffer_size = gpu_buffer.read().gpu().byte_size();
    let buffer_size = inspector.format_readable_data_size(buffer_size);
    inspector.label(&format!("bindless index, size: {}", buffer_size));
  }

  let index_buffer_ref = cx.use_dual_query::<SceneBufferViewBufferId<AttributeIndexRef>>();
  let index_buffer_range = cx.use_dual_query::<SceneBufferViewBufferRange<AttributeIndexRef>>();
  let index_item_count = cx.use_dual_query::<SceneBufferViewBufferItemCount<AttributeIndexRef>>();

  let source_info = index_buffer_ref
    .dual_query_union(index_buffer_range, |(a, b)| Some((a?, b?)))
    .dual_query_zip(index_item_count)
    .dual_query_filter_map(|((index, range), count)| index.map(|i| (i, range, count)))
    .dual_query_boxed();

  let (cx, allocator) =
    cx.use_sharable_plain_state(|| GrowableRangeAllocator::new(max_item_count, init_item_count));

  let allocator = allocator.clone();
  let gpu_buffer_ = gpu_buffer.clone();

  let allocation_info = source_info.map_spawn_stage_in_thread_dual_query(cx, move |dual| {
    let change = dual.delta().into_change();
    let removed_and_changed_keys = change
      .iter_removed()
      .chain(change.iter_update_or_insert().map(|(k, _)| k));

    let data = get_db_view::<BufferEntityData>();

    // todo, avoid resize
    let mut buffers_to_write = RangeAllocateBufferCollector::default();
    let mut new_sizes = Vec::new();

    for (k, (buffer_id, range, count)) in change.iter_update_or_insert() {
      let buffer = data.read_ref(buffer_id).unwrap().ptr.clone();

      let range = range.map(|range| range.into_range(buffer.len()));

      let byte_per_item = buffer.len() / count as usize;
      if byte_per_item != 4 && byte_per_item != 2 {
        unreachable!("index count must be multiple of 2(u16) or 4(u32)")
      }

      let buffer = if byte_per_item == 2 {
        let mut buffer = buffer.as_slice();
        if let Some(range) = range {
          buffer = &buffer[range];
        }
        let buffer = bytemuck::cast_slice::<_, u16>(buffer);
        let buffer = buffer.iter().map(|i| *i as u32).collect::<Vec<_>>();
        let buffer = bytemuck::cast_slice(&buffer).to_vec();
        (Arc::new(buffer), None)
      } else {
        (buffer, range)
      };

      let size = buffer.1.clone().map(|v| v.len()).unwrap_or(buffer.0.len()) as u32 / 4;
      buffers_to_write.collect_shared(k, buffer);
      new_sizes.push((k, size));
    }

    let changes = allocator
      .write()
      .update(removed_and_changed_keys, new_sizes);

    let buffers_to_write = buffers_to_write.prepare(&changes, 4);

    if let Some(new_size) = changes.resize_to {
      // here we do(request) resize at spawn stage to avoid resize again and again
      gpu_buffer_.write().resize(new_size);
    }

    Arc::new(RangeAllocateBufferUpdates {
      buffers_to_write,
      allocation_changes: BatchAllocateResultShared(Arc::new(changes), 1),
    })
  });

  let (allocation_info, allocation_info_) = allocation_info.fork();

  let allocation_info_ = allocation_info_.use_assure_result(cx);

  if let GPUQueryHookStage::CreateRender { encoder, .. } = &mut cx.stage {
    let mut gpu_buffer = gpu_buffer.write();
    let gpu_buffer = gpu_buffer.abstract_gpu();
    allocation_info_
      .expect_resolve_stage()
      .write(cx.gpu, encoder, gpu_buffer);
  }

  let changes = allocation_info.map(|v| v.allocation_changes.clone());
  let buffer = gpu_buffer.read().gpu().clone();
  (changes, buffer)
}

fn use_attribute_vertex_updates(
  cx: &mut QueryGPUHookCx,
  max_item_count: u32,
  init_item_count: u32,
  semantic: AttributeSemantic,
) -> (
  UseResult<impl DataChanges<Key = RawEntityHandle, Value = [u32; 2]> + 'static>,
  AbstractReadonlyStorageBuffer<[u32]>,
) {
  let item_byte_size = semantic.item_byte_size() as u32;
  let (cx, vertex_buffer) = cx.use_gpu_init(|gpu, alloc| {
    let buffer = alloc.allocate_readonly::<[u32]>(
      (item_byte_size * init_item_count) as u64,
      &gpu.device,
      Some(&format!("bindless mesh vertex pool: {:?}", semantic)),
    );

    let buffer = buffer.with_direct_resize(gpu);

    Arc::new(RwLock::new(buffer))
  });

  if let GPUQueryHookStage::Inspect(inspector) = &mut cx.stage {
    let buffer_size = vertex_buffer.read().gpu().byte_size();
    let buffer_size = inspector.format_readable_data_size(buffer_size);
    inspector.label(&format!("bindless {:?}, size: {}", semantic, buffer_size));
  }

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
    .dual_query_filter_by_set(scope);

  // todo, share one allocator among all vertex buffer
  let (cx, allocator) =
    cx.use_sharable_plain_state(|| GrowableRangeAllocator::new(max_item_count, init_item_count));

  let allocator = allocator.clone();
  let gpu_buffer = vertex_buffer.clone();

  let allocation_info = source_info.map_spawn_stage_in_thread_dual_query(cx, move |source_info| {
    let change = source_info.delta().into_change();
    let removed_and_changed_keys = change
      .iter_removed()
      .chain(change.iter_update_or_insert().map(|(k, _)| k));

    let data = get_db_view::<BufferEntityData>();

    // todo, avoid resize
    let mut buffers_to_write = RangeAllocateBufferCollector::default();
    let mut sizes = Vec::new();

    for (k, (buffer_id, range)) in change.iter_update_or_insert() {
      let buffer = data.read_ref(buffer_id).unwrap().ptr.clone();

      let range = range.map(|range| range.into_range(buffer.len()));

      let len = range
        .clone()
        .map(|range| range.len() as u32)
        .unwrap_or(buffer.len() as u32);
      buffers_to_write.collect_shared(k, (buffer, range));
      sizes.push((k, len / item_byte_size));
    }

    let changes = allocator.write().update(removed_and_changed_keys, sizes);

    let buffers_to_write = buffers_to_write.prepare(&changes, item_byte_size);

    if let Some(new_size) = changes.resize_to {
      // here we do(request) resize at spawn stage to avoid resize again and again
      gpu_buffer.write().resize(new_size * item_byte_size / 4);
    }

    Arc::new(RangeAllocateBufferUpdates {
      buffers_to_write,
      allocation_changes: BatchAllocateResultShared(Arc::new(changes), item_byte_size / 4),
    })
  });

  let (allocation_info, allocation_info_) = allocation_info.fork();

  let allocation_info_ = allocation_info_.use_assure_result(cx);
  if let GPUQueryHookStage::CreateRender { encoder, .. } = &mut cx.stage {
    let mut gpu_buffer = vertex_buffer.write();
    let gpu_buffer = gpu_buffer.abstract_gpu();
    allocation_info_
      .expect_resolve_stage()
      .write(cx.gpu, encoder, gpu_buffer);
  }

  let ab_ref_mesh = cx
    .use_dual_query::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .dual_query_filter_map(|v| v)
    .dual_query_filter_by_set(scope_)
    .use_dual_query_hash_reverse_checked_one_one(cx)
    .dual_query_boxed()
    .use_dual_query_dense_many_to_one(cx);

  let allocation_info = allocation_info
    .map(|allocation_info| allocation_info.allocation_changes.clone())
    .use_change_to_dual_query_in_spawn_stage(cx);

  let change = allocation_info
    .fanout(ab_ref_mesh, cx)
    .dual_query_boxed()
    .into_delta_change();

  let buffer = vertex_buffer.read().gpu().clone();

  (change, buffer)
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

#[derive(Clone)]
pub struct MeshGPUBindlessImpl {
  indices: AbstractReadonlyStorageBuffer<[u32]>,
  position: AbstractReadonlyStorageBuffer<[u32]>,
  normal: AbstractReadonlyStorageBuffer<[u32]>,
  uv: AbstractReadonlyStorageBuffer<[u32]>,
  vertex_address_buffer: AbstractReadonlyStorageBuffer<[AttributeMeshMeta]>,
  /// we keep the host metadata to support creating draw commands from host
  vertex_address_buffer_host:
    LockReadGuardHolder<SparseStorageBufferWithHostRaw<AttributeMeshMeta>>,
  sm_to_mesh_device: AbstractReadonlyStorageBuffer<[u32]>,
  sm_to_mesh: BoxedDynQuery<RawEntityHandle, RawEntityHandle>,
  checker: ForeignKeyReadView<StandardModelRefAttributesMeshEntity>,
  indices_checker: ForeignKeyReadView<SceneBufferViewBufferId<AttributeIndexRef>>,
  topology_checker: ComponentReadView<AttributesMeshEntityTopology>,
  used_in_midc_downgrade: bool,
}

impl MeshGPUBindlessImpl {
  pub fn make_bindless_dispatcher(&self) -> BindlessMeshDispatcher {
    let position = self.position.clone();
    let normal = self.normal.clone();
    let uv = self.uv.clone();

    let index_pool = self.indices.clone();

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
        batch.create_default_indirect_draw_provider(
          draw_command_builder,
          cx,
          self.used_in_midc_downgrade,
        )
      })
      .into()
  }
}

#[derive(Clone)]
pub struct BindlessMeshDispatcher {
  pub sm_to_mesh: AbstractReadonlyStorageBuffer<[u32]>,
  pub vertex_address_buffer: AbstractReadonlyStorageBuffer<[AttributeMeshMeta]>,
  pub index_pool: AbstractReadonlyStorageBuffer<[u32]>,
  pub position: AbstractReadonlyStorageBuffer<[u32]>,
  pub normal: AbstractReadonlyStorageBuffer<[u32]>,
  pub uv: AbstractReadonlyStorageBuffer<[u32]>,
}

impl ShaderHashProvider for BindlessMeshDispatcher {
  shader_hash_type_id! {}
}

#[derive(Clone)]
pub struct BindlessMeshRasterDispatcher {
  pub internal: BindlessMeshDispatcher,
  pub is_indexed: bool,
  pub topology: rendiation_webgpu::PrimitiveTopology,
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
      // may be failed if we using texture as storage
      if let Some(index) = mesh.index_pool.get_gpu_buffer_view() {
        ctx
          .pass
          .set_index_buffer_by_buffer_resource_view(&index, IndexFormat::Uint32);
      }
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
  metadata: AbstractReadonlyStorageBuffer<[AttributeMeshMeta]>,
  sm_to_mesh: BoxedDynQuery<RawEntityHandle, RawEntityHandle>,
  sm_to_mesh_device: AbstractReadonlyStorageBuffer<[u32]>,
  vertex_address_buffer_host:
    LockReadGuardHolder<SparseStorageBufferWithHostRaw<AttributeMeshMeta>>,
}
impl NoneIndexedDrawCommandBuilder for BindlessDrawCreator {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> DrawCommand {
    let mesh_id = self.sm_to_mesh.access(&id.into_raw()).unwrap();
    let address_info = self
      .vertex_address_buffer_host
      .get(mesh_id.alloc_index())
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
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> DrawCommand {
    let mesh_id = self.sm_to_mesh.access(&id.into_raw()).unwrap();
    let address_info = self
      .vertex_address_buffer_host
      .get(mesh_id.alloc_index())
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
    // shader_assert(meta.index_offset.not_equals(val(u32::MAX)));

    let meta = self.metadata.index(mesh_handle).load().expand();
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

    let meta = self.metadata.index(mesh_handle).load().expand();
    ENode::<DrawIndirectArgsStorage> {
      vertex_count: meta.position_count / val(3),
      instance_count: val(1),
      base_vertex: val(0),
      base_instance: draw_id,
    }
    .construct()
  }
}
