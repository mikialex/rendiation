use std::{mem::offset_of, sync::Arc};

use parking_lot::RwLock;
use rendiation_mesh_core::AttributeSemantic;
use rendiation_shader_api::*;
use rendiation_webgpu_midc_downgrade::*;

mod draw_cmd;
pub use draw_cmd::*;

mod render;
pub use render::*;

only_vertex!(IndirectAbstractMeshId, u32);

use crate::*;

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct BindlessMeshInit {
  pub init_index_count: u32,
  pub max_index_count: u32,
  pub init_vertex_u32_size_count: u32,
  pub max_vertex_u32_size_count: u32,
}

impl Default for BindlessMeshInit {
  fn default() -> Self {
    Self {
      init_index_count: 200_000,
      max_index_count: 200_000 * 100,
      init_vertex_u32_size_count: 100_000 * 8, // 8: 3+3+2
      max_vertex_u32_size_count: 100_000 * 8 * 100,
    }
  }
}

pub fn use_bindless_mesh(
  cx: &mut QueryGPUHookCx,
  init: &BindlessMeshInit,
  merge_with_vertex_allocator: bool,
  use_midc_downgrade: bool,
  index_data_source: AttributeIndexDataSource,
  vertex_data_source: AttributeVertexDataSource,
) -> Option<MeshGPUBindlessImpl> {
  let force_midc_downgrade = use_midc_downgrade || merge_with_vertex_allocator;

  let BindlessMeshInit {
    init_index_count,
    max_index_count,
    init_vertex_u32_size_count,
    max_vertex_u32_size_count,
  } = *init;

  let (indices_range_change, indices) = use_attribute_indices_updates(
    cx,
    max_index_count,
    init_index_count,
    merge_with_vertex_allocator,
    index_data_source,
  );

  let (cx, metadata) = cx.use_storage_buffer_with_host_backup::<AttributeMeshMeta>(
    "mesh buffer indirect range",
    128,
    u32::MAX,
  );

  let max = max_vertex_u32_size_count;
  let init = init_vertex_u32_size_count;
  let (vertex_range_writes, vertices) =
    use_attribute_vertex_updates(cx, max, init, vertex_data_source);

  let offset = offset_of!(AttributeMeshMeta, index_offset);
  indices_range_change.update_storage_array_with_host(cx, metadata, offset);

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

  let vertex_range_writes = vertex_range_writes.use_assure_result(cx);
  if let GPUQueryHookStage::CreateRender { encoder, .. } = &mut cx.stage {
    {
      let updates = vertex_range_writes.expect_resolve_stage();
      updates.write_abstract(cx.gpu, encoder, &metadata.get_gpu_buffer());
      metadata.write_sparse_updates(&updates);
    }
  }

  let sm_to_mesh = fanout_
    .map(|v| v.view().filter_map(|v| v).into_boxed())
    .use_assure_result(cx);

  cx.when_render(|| {
    let vertex_address_buffer = metadata.get_gpu_buffer();
    MeshGPUBindlessImpl {
      indices,
      vertices,
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
  index_source: AttributeIndexDataSource,
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

  cx.if_inspect(|inspector| {
    let buffer_size = gpu_buffer.read().gpu().byte_size();
    inspector.label_device_memory_usage("bindless index", buffer_size);
  });

  let (cx, allocator) =
    cx.use_sharable_plain_state(|| GrowableRangeAllocator::new(max_item_count, init_item_count));

  let gpu_buffer_ = gpu_buffer.clone();

  let allocation_info = index_source.map_spawn_stage_in_thread_data_changes(cx, move |change| {
    let removed_and_changed_keys = change
      .iter_removed()
      .chain(change.iter_update_or_insert().map(|(k, _)| k));

    let data = get_db_view::<BufferEntityData>();

    // todo, avoid resize
    let mut buffers_to_write = RangeAllocateBufferCollector::default();
    let mut new_sizes = Vec::new();

    for (k, (buffer_id, range, count)) in change.iter_update_or_insert() {
      let buffer = &data.read_ref(buffer_id).unwrap().ptr;
      let buffer = buffer.as_living().unwrap();

      let range = range.map(|range| range.into_range(buffer.len()));

      let byte_per_item = buffer.len() / count as usize;
      if byte_per_item != 4 && byte_per_item != 2 {
        unreachable!("index count must be multiple of 2(u16) or 4(u32)")
      }

      if byte_per_item == 2 {
        let mut buffer = buffer.as_slice();
        if let Some(range) = range {
          buffer = &buffer[range];
        }
        let buffer = bytemuck::cast_slice::<_, u16>(buffer);
        let buffer = buffer.iter().map(|i| *i as u32).collect::<Vec<_>>();
        let buffer = bytemuck::cast_slice(&buffer).to_vec();
        let buffer = Arc::new(buffer);

        let size = buffer.len() as u32 / 4;
        buffers_to_write.collect_shared(k, (&buffer, None));
        new_sizes.push((k, size));
      } else {
        let size = range.clone().map(|v| v.len()).unwrap_or(buffer.len()) as u32 / 4;
        buffers_to_write.collect_shared(k, (buffer, range));
        new_sizes.push((k, size));
      };
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

pub const ENABLE_VERTEX_RANGE_UPDATE_DEBUG: bool = false;

fn use_attribute_vertex_updates(
  cx: &mut QueryGPUHookCx,
  max_u32_count: u32,
  init_u32_count: u32,
  vertex_data_source: AttributeVertexDataSource,
) -> (
  UseResult<Arc<SparseBufferWritesSource>>,
  AbstractReadonlyStorageBuffer<[u32]>,
) {
  let (cx, vertex_buffer) = cx.use_gpu_init(|gpu, alloc| {
    let buffer = alloc.allocate_readonly::<[u32]>(
      init_u32_count as u64 * 4,
      &gpu.device,
      Some("bindless mesh vertex pool"),
    );

    let buffer = buffer.with_direct_resize(gpu);

    Arc::new(RwLock::new(buffer))
  });

  cx.if_inspect(|inspector| {
    let buffer_size = vertex_buffer.read().gpu().byte_size();
    inspector.label_device_memory_usage("bindless vertex pool", buffer_size);
  });

  let (cx, allocator) =
    cx.use_sharable_plain_state(|| GrowableRangeAllocator::new(max_u32_count, init_u32_count));

  let gpu_buffer = vertex_buffer.clone();

  let allocation_info =
    vertex_data_source.map_spawn_stage_in_thread_data_changes(cx, move |change| {
      let data = get_db_view::<BufferEntityData>();

      // todo, this code should be improved
      let mut small_buffer_count = 0;
      let mut small_buffer_byte_count = 0;
      let mut large_buffer_count = 0;

      let iter = change.iter_update_or_insert();
      let size_hint = iter.size_hint();
      // use conservative hint because we have filter in upstream
      let size_cap = size_hint.1.unwrap_or(size_hint.0);
      let mut sizes = Vec::with_capacity(size_cap);

      // iter is slow to iter, do this is much faster
      let mut access_result = Vec::with_capacity(size_cap);
      for (k, (buffer_id, range)) in iter {
        let buffer = &data.read_ref(buffer_id).unwrap().ptr;
        let buffer = buffer.as_living().unwrap();
        let range = range.map(|range| range.into_range(buffer.len()));
        let len = range
          .clone()
          .map(|range| range.len())
          .unwrap_or(buffer.len());

        if len <= SMALL_BUFFER_THRESHOLD_BYTE_COUNT {
          small_buffer_count += 1;
          small_buffer_byte_count += len;
        } else {
          large_buffer_count += 1;
        }

        sizes.push((k, len as u32 / 4));
        access_result.push((k, buffer, range));
      }

      let removed_and_changed_keys = change
        .iter_removed()
        .chain(access_result.iter().map(|v| v.0));
      let changes = allocator.write().update(removed_and_changed_keys, sizes);

      let mut buffers_to_write = RangeAllocateBufferCollector::with_capacity(
        small_buffer_byte_count,
        small_buffer_count,
        large_buffer_count,
      );

      for (k, buffer, range) in access_result {
        buffers_to_write.collect_shared(k, (buffer, range));
      }

      let buffers_to_write = buffers_to_write.prepare(&changes, 4);

      if let Some(new_size) = changes.resize_to {
        // here we do(request) resize at spawn stage to avoid resize again and again
        gpu_buffer.write().resize(new_size);
      }

      Arc::new(RangeAllocateBufferUpdates {
        buffers_to_write,
        allocation_changes: BatchAllocateResultShared(Arc::new(changes), 1),
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

  // relation => mesh
  let vertex_buffer_sem = cx.use_dual_query::<AttributesMeshEntityVertexBufferSemantic>();
  let relation_ref_mesh = cx
    .use_dual_query::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .dual_query_zip(vertex_buffer_sem);

  // relation => allocation info
  let allocation_info =
    allocation_info.map(|allocation_info| allocation_info.allocation_changes.clone());

  let range_writes = relation_ref_mesh
    .join(allocation_info)
    .map_spawn_stage_in_thread(
      cx,
      |(ref_change, alloc_change)| ref_change.has_delta_hint() || alloc_change.has_change(),
      |(ref_side, alloc_side)| {
        let (ref_view, ref_change) = ref_side.view_delta();
        let alloc_delta_iter = alloc_side.iter_update_or_insert();
        let ref_change_iter = ref_change.iter_key_value();
        let change_estimate = alloc_delta_iter.size_hint().0 + ref_change_iter.size_hint().0;
        let mut writes = FastHashMap::with_capacity_and_hasher(change_estimate, Default::default());
        // we are not care removes here, because failed allocated range will have correct defaults
        // todo, assure the mesh is valid and skip the invalid mesh.
        for (k, new) in alloc_delta_iter {
          if let Some((Some(mesh), se)) = ref_view.access(&k) {
            writes.insert((mesh, se), new);
          }
        }

        for (k, v) in ref_change_iter {
          if let Some(range) = alloc_side.access_new_change(k) {
            if let ValueChange::Delta((Some(new_mesh), se), _) = v {
              writes.insert((new_mesh, se), range);
            }
          }
        }

        let data_write_size = writes.len() * std::mem::size_of::<[u32; 2]>();
        let mut updates = SparseBufferWritesSource::with_capacity(data_write_size, writes.len());

        let stride = std::mem::size_of::<AttributeMeshMeta>() as u32;
        for ((mesh, se), range) in writes {
          if ENABLE_VERTEX_RANGE_UPDATE_DEBUG {
            println!("{:?}, {:?}, {:?}", mesh, se, range);
          }
          if let Some(field_offset) = write_field_offset(se) {
            let write_offset = stride * mesh.index() + field_offset;
            updates.collect_write(bytes_of(&range), write_offset as u64);
          }
        }

        Arc::new(updates)
      },
    );

  (range_writes, vertex_buffer.read().gpu().clone())
}

fn write_field_offset(semantic: AttributeSemantic) -> Option<u32> {
  let offset = match semantic {
    AttributeSemantic::Positions => std::mem::offset_of!(AttributeMeshMeta, position_offset),
    AttributeSemantic::Normals => std::mem::offset_of!(AttributeMeshMeta, normal_offset),
    AttributeSemantic::TexCoords(0) => std::mem::offset_of!(AttributeMeshMeta, uv_offset),
    _ => return None,
  };
  Some(offset as u32)
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
  vertices: AbstractReadonlyStorageBuffer<[u32]>,
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
    BindlessMeshDispatcher {
      sm_to_mesh: self.sm_to_mesh_device.clone(),
      vertex_address_buffer: self.vertex_address_buffer.clone(),
      vertices: self.vertices.clone(),
      index_pool: self.indices.clone(),
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
