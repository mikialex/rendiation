use std::{hash::Hash, mem::offset_of, sync::Arc};

use parking_lot::RwLock;
use rendiation_mesh_core::AttributeSemantic;

mod draw_cmd;
pub use draw_cmd::*;

mod render;
pub use render::*;

only_vertex!(IndirectAbstractMeshId, u32);

use crate::*;

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct IndirectAttributeMeshInitConfig {
  pub init_index_count: u32,
  pub max_index_count: u32,
  pub init_vertex_u32_size_count: u32,
  pub max_vertex_u32_size_count: u32,
  /// if enabled, the normal data will be treated as octahedral quantized [u32] instead of [Vec3<f32>]
  pub enable_normal_quantization: bool,
  /// if enabled, the normal data will be convert to octahedral quantized when upload to gpu, expect
  /// input normal be [Vec3<f32>]
  pub enable_normal_quantization_convert: bool,
}

impl Default for IndirectAttributeMeshInitConfig {
  fn default() -> Self {
    Self {
      init_index_count: 200_000,
      max_index_count: 200_000 * 100,
      init_vertex_u32_size_count: 100_000 * 8, // 8: 3+3+2
      max_vertex_u32_size_count: 100_000 * 8 * 100,
      enable_normal_quantization: false,
      enable_normal_quantization_convert: false,
    }
  }
}

pub fn use_attribute_mesh_indirect_renderer(
  cx: &mut QueryGPUHookCx,
  init: &IndirectAttributeMeshInitConfig,
  merge_with_vertex_allocator: bool,
  force_midc_downgrade: bool,
  mesh_changes: UseResult<AttributesMeshDataChangeInput>,
) -> Option<AttributeMeshIndirectRenderer> {
  let (vertex_data_source, index_data_source) =
    create_sub_buffer_changes_from_mesh_changes(cx, mesh_changes);

  let force_midc_downgrade = force_midc_downgrade || merge_with_vertex_allocator;

  let IndirectAttributeMeshInitConfig {
    init_index_count,
    max_index_count,
    init_vertex_u32_size_count,
    max_vertex_u32_size_count,
    enable_normal_quantization,
    enable_normal_quantization_convert,
  } = *init;

  let (index_data_source, index_data_source_) = index_data_source.fork();
  let (index_data_source_, index_data_source__) = index_data_source_.fork();

  let indices_ty = index_data_source_
    .map_changes(|data| {
      let byte_size = data.byte_view().len();
      let byte_per_item = byte_size / data.count;
      assert!(byte_size.is_multiple_of(data.count));
      if byte_per_item == 2 {
        IndexFormat::Uint16
      } else if byte_per_item == 4 {
        IndexFormat::Uint32
      } else {
        unreachable!("index count must be multiple of 2(u16) or 4(u32)")
      }
    })
    .use_change_to_dual_query_in_spawn_stage(cx)
    .dual_query_boxed()
    .use_assure_result(cx);

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

  let indices_marker = index_data_source__.map_changes(|data| {
    let byte_size = data.byte_view().len();
    let byte_per_item = byte_size / data.count;
    assert!(byte_size.is_multiple_of(data.count));
    let is_u16 = Bool::from(byte_per_item == 2);
    let padded = Bool::from(!byte_size.is_multiple_of(4));

    [is_u16, padded]
  });

  // note, if the mesh change from index to none indexed, this flag in gpu will not be update, but it's ok
  // as the logic should not access this data on device anymore.
  let offset = offset_of!(AttributeMeshMeta, is_u16_indices);
  indices_marker.update_storage_array_with_host(cx, metadata, offset);

  let max = max_vertex_u32_size_count;
  let init = init_vertex_u32_size_count;
  let (vertex_range_writes, vertices) = use_attribute_vertex_updates(
    cx,
    max,
    init,
    vertex_data_source,
    enable_normal_quantization_convert,
  );

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
    AttributeMeshIndirectRenderer {
      indices,
      vertices,
      std_to_mesh: read_global_db_foreign_key(),
      indices_ty: indices_ty.expect_resolve_stage().view,
      topology: read_global_db_component(),
      vertex_address_buffer,
      vertex_address_buffer_host: metadata.buffer.make_read_holder(),
      sm_to_mesh_device: sm_to_mesh_device.get_gpu_buffer(),
      sm_to_mesh: sm_to_mesh.expect_resolve_stage(),
      used_in_midc_downgrade: require_midc_downgrade(&cx.gpu.info, force_midc_downgrade),
      enable_normal_quantization,
    }
  })
}

pub fn use_attribute_mesh_indirect_render_vertex_count(
  cx: &mut impl DBHookCxLike,
  mesh_changes: UseResult<AttributesMeshDataChangeInput>,
) -> UseResult<BoxedDynDualQuery<RawEntityHandle, u32>> {
  mesh_changes
    .filter_map_changes(|v| v.if_loaded().map(|v| v.vertices_count() as u32))
    .use_change_to_dual_query_in_spawn_stage(cx)
    .fanout(
      cx.use_db_rev_ref_tri_view::<StandardModelRefAttributesMeshEntity>(),
      cx,
    )
    .fanout(
      cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>(),
      cx,
    )
    .dual_query_boxed()
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
        "indirect attribute mesh index pool",
      )
    } else {
      StorageBufferReadonlyDataView::<[u32]>::create_by_with_extra_usage(
        &gpu.device,
        ZeroedArrayByArrayLength(init_item_count as usize).into(),
        BufferUsages::INDEX,
        "indirect attribute mesh index pool",
      )
      .into()
    };

    let indices = indices.with_direct_resize(gpu);

    Arc::new(RwLock::new(indices))
  });

  let label = "indirect mesh indices";

  cx.if_inspect(|inspector| {
    let buffer_size = gpu_buffer.read().gpu().byte_size();
    inspector.label_device_memory_usage(label, buffer_size);
  });

  let allocator = cx.use_sharable_plain_state(|| {
    GrowableRangeAllocator::new(label, max_item_count, init_item_count, 1)
  });

  let gpu_buffer_ = gpu_buffer.clone();

  let allocation_info = index_source.map_spawn_stage_in_thread_data_changes(cx, move |change| {
    let removed_and_changed_keys = change
      .iter_removed()
      .chain(change.iter_update_or_insert().map(|(k, _)| k));

    // todo, avoid resize
    let mut buffers_to_write = RangeAllocateBufferCollector::default();
    let mut new_sizes = Vec::new();

    for (k, data) in change.iter_update_or_insert() {
      let range = data.range.map(|range| range.into_range(data.data.len()));

      let byte_size = data.byte_view().len();
      let byte_per_item = byte_size / data.count;

      let mut allocate_request_u32_size = byte_size as u32 / 4;
      // the upload path requires 4-byte aligned chunks, pad the data tail with 2 zero bytes
      // to fill the extra u32 allocated above
      let padded_shared_buffer;
      let (buffer, range) = if byte_per_item == 2 && !byte_size.is_multiple_of(4) {
        allocate_request_u32_size += 1;
        let mut padded = data.byte_view().to_vec();
        padded.resize(padded.len().next_multiple_of(4), 0);
        padded_shared_buffer = Some(Arc::new(padded));
        (padded_shared_buffer.as_ref().unwrap(), None)
      } else {
        (&data.data, range)
      };
      buffers_to_write.collect_shared(k, (buffer, range));
      new_sizes.push((k, allocate_request_u32_size));
    }

    let changes = allocator
      .write()
      .update(removed_and_changed_keys, new_sizes);

    let buffers_to_write = buffers_to_write.prepare(&changes, 4);

    let allocation_changes = BatchAllocateResultShared::new(changes, 1);
    allocation_changes.apply_resize(&mut *gpu_buffer_.write());

    Arc::new(RangeAllocateBufferUpdates {
      buffers_to_write,
      allocation_changes,
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

/// return (each vertex writes, vertex buffer)
fn use_attribute_vertex_updates(
  cx: &mut QueryGPUHookCx,
  max_u32_count: u32,
  init_u32_count: u32,
  vertex_data_source: AttributeVertexDataSource,
  enable_normal_quantization_convert: bool,
) -> (
  UseResult<Arc<SparseBufferWritesSource>>,
  AbstractReadonlyStorageBuffer<[u32]>,
) {
  let label = "indirect mesh vertices";
  let (cx, vertex_buffer) = cx.use_gpu_init(|gpu, alloc| {
    let buffer = alloc.allocate_readonly::<[u32]>(init_u32_count as u64 * 4, &gpu.device, label);

    let buffer = buffer.with_direct_resize(gpu);

    Arc::new(RwLock::new(buffer))
  });

  cx.if_inspect(|inspector| {
    let buffer_size = vertex_buffer.read().gpu().byte_size();
    inspector.label_device_memory_usage(label, buffer_size);
  });

  let allocator = cx.use_sharable_plain_state(|| {
    GrowableRangeAllocator::new(label, max_u32_count, init_u32_count, 1)
  });

  let gpu_buffer = vertex_buffer.clone();

  let allocation_info =
    vertex_data_source.map_spawn_stage_in_thread_data_changes(cx, move |change| {
      // todo, this code should be improved
      // we should add datachange ref trait to avoid some arc clone
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
      for (k, (data, semantic)) in iter {
        let range = data.range.map(|range| range.into_range(data.data.len()));
        let len = range
          .clone()
          .map(|range| range.len())
          .unwrap_or(data.data.len());

        let len = if enable_normal_quantization_convert && semantic == AttributeSemantic::Normals {
          len / 3
        } else {
          len
        };

        if len <= SMALL_BUFFER_THRESHOLD_BYTE_COUNT {
          small_buffer_count += 1;
          small_buffer_byte_count += len;
        } else {
          large_buffer_count += 1;
        }

        sizes.push((k, len as u32 / 4));

        if enable_normal_quantization_convert && semantic == AttributeSemantic::Normals {
          let data = data.data.as_slice();
          let data = if let Some(range) = range {
            data.get(range).unwrap()
          } else {
            data
          };
          let normal: &[Vec3<f32>] = bytemuck::cast_slice(data);
          let mut quantized_bytes: Vec<u8> = Vec::with_capacity(normal.len() * 4);
          for v in normal {
            let quantized = rendiation_shader_library::octahedral::encode_octahedral_normal(*v);
            quantized_bytes.extend_from_slice(bytes_of(&quantized));
          }
          access_result.push((k, Arc::new(quantized_bytes), None));
        } else {
          access_result.push((k, data.data, range));
        };
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
        buffers_to_write.collect_shared(k, (&buffer, range));
      }

      let buffers_to_write = buffers_to_write.prepare(&changes, 4);

      let allocation_changes = BatchAllocateResultShared::new(changes, 1);
      allocation_changes.apply_resize(&mut *gpu_buffer.write());

      Arc::new(RangeAllocateBufferUpdates {
        buffers_to_write,
        allocation_changes,
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
  pub is_u16_indices: Bool,
  // set when the u16 index data is not 4-byte aligned, the data tail is padded to
  // fill the last allocated u32 which then holds only one real index
  pub is_u16_indices_padded: Bool,
  pub position_offset: u32,
  pub position_count: u32,
  pub normal_offset: u32,
  pub normal_count: u32,
  pub uv_offset: u32,
  pub uv_count: u32,
}

#[derive(Clone)]
pub struct AttributeMeshIndirectRenderer {
  pub indices: AbstractReadonlyStorageBuffer<[u32]>,
  pub vertices: AbstractReadonlyStorageBuffer<[u32]>,
  pub vertex_address_buffer: AbstractReadonlyStorageBuffer<[AttributeMeshMeta]>,
  /// we keep the host metadata to support creating draw commands from host
  pub vertex_address_buffer_host:
    LockReadGuardHolder<SparseStorageBufferWithHostRaw<AttributeMeshMeta>>,
  pub sm_to_mesh_device: AbstractReadonlyStorageBuffer<[u32]>,
  pub sm_to_mesh: BoxedDynQuery<RawEntityHandle, RawEntityHandle>,
  pub std_to_mesh: ForeignKeyReadView<StandardModelRefAttributesMeshEntity>,
  /// mesh id => indices type (None is not indexed)
  pub indices_ty: BoxedDynQuery<RawEntityHandle, IndexFormat>,
  pub topology: ComponentReadView<AttributesMeshEntityTopology>,
  pub used_in_midc_downgrade: bool,
  pub enable_normal_quantization: bool,
}

impl AttributeMeshIndirectRenderer {
  pub fn make_dispatcher(&self) -> AttributeMeshIndirectDispatcher {
    AttributeMeshIndirectDispatcher {
      sm_to_mesh: self.sm_to_mesh_device.clone(),
      vertex_address_buffer: self.vertex_address_buffer.clone(),
      vertices: self.vertices.clone(),
      index_pool: self.indices.clone(),
      enable_normal_quantization: self.enable_normal_quantization,
    }
  }
}

impl IndirectDrawProviderCreator for AttributeMeshIndirectRenderer {
  fn get_impl_distinguish_key_by_impl_select_id(&self, id: RawEntityHandle) -> Option<u64> {
    let id = unsafe { EntityHandle::from_raw(id) };
    let mesh_id = self.std_to_mesh.get(id)?;
    let indices_ty = self.indices_ty.access(mesh_id.raw_handle_ref());
    fast_hash_scope(|hasher| {
      self.type_id().hash(hasher);
      // index type not matters
      indices_ty.is_some().hash(hasher);
    })
    .into()
  }

  fn use_create_or_update_indirect_draw_providers(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    list: &DeviceDrawList,
    dispatch_info_device_offset_compacted: &MultiRangeDispatchInfo,
    id: RawEntityHandle,
  ) -> Option<Vec<Box<dyn IndirectDrawProvider>>> {
    let cmd_builder = self.make_draw_command_builder(id)?;
    use_and_create_default_indirect_draw_provider(
      list,
      dispatch_info_device_offset_compacted,
      cmd_builder,
      cx,
      self.used_in_midc_downgrade,
    )
    .into()
  }
}

impl AttributeMeshIndirectRenderer {
  pub fn make_draw_command_builder_impl(
    &self,
    id: RawEntityHandle,
  ) -> Option<(AttributeMeshIndirectDrawCreator, bool)> {
    let id = unsafe { EntityHandle::from_raw(id) };
    let mesh_id = self.std_to_mesh.get(id)?;
    let is_indexed = self.indices_ty.access(mesh_id.raw_handle_ref()).is_some();

    let creator = AttributeMeshIndirectDrawCreator {
      metadata: self.vertex_address_buffer.clone(),
      sm_to_mesh_device: self.sm_to_mesh_device.clone(),
      sm_to_mesh: self.sm_to_mesh.clone(),
      vertex_address_buffer_host: self.vertex_address_buffer_host.clone(),
      used_in_midc_downgrade: self.used_in_midc_downgrade,
    };
    (creator, is_indexed).into()
  }
}

impl DrawCommandBuilderCreator for AttributeMeshIndirectRenderer {
  fn make_draw_command_builder(&self, id: RawEntityHandle) -> Option<DrawCommandBuilder> {
    let (creator, is_indexed) = self.make_draw_command_builder_impl(id)?;

    if is_indexed {
      DrawCommandBuilder::Indexed(Box::new(creator))
    } else {
      DrawCommandBuilder::NoneIndexed(Box::new(creator))
    }
    .into()
  }
}

impl IndirectModelShapeRenderImpl for AttributeMeshIndirectRenderer {
  fn make_component_indirect(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    // check the given model has attributes mesh
    let mesh = self.std_to_mesh.get(any_idx)?;
    let indices_ty = self.indices_ty.access(mesh.raw_handle_ref());
    let topology = self.topology.get(mesh)?;

    let mesh_system = AttributeMeshIndirectRasterDispatcher {
      internal: self.make_dispatcher(),
      topology: map_topology(*topology),
      indices_ty,
    };

    Some(Box::new(mesh_system))
  }

  fn get_index_storage_buffer(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Option<IndicesBufferInfo>> {
    let mesh_id = self.std_to_mesh.get(any_idx)?;
    let indices_ty = self.indices_ty.access(mesh_id.raw_handle_ref());
    if let Some(indices_ty) = indices_ty {
      Some(IndicesBufferInfo {
        buffer: self.indices.clone(),
        should_access_as_u16: matches!(indices_ty, IndexFormat::Uint16),
      })
    } else {
      None
    }
    .into()
  }

  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    let mesh_id = self.std_to_mesh.get(any_id)?;
    let topology = self.topology.get(mesh_id)?;
    hasher.hash(topology);
    let indices_ty = self.indices_ty.access(mesh_id.raw_handle_ref());
    hasher.hash(indices_ty);
    // enable_normal_quantization is not mutable at runtime, so hash can be skipped
    Some(())
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}
