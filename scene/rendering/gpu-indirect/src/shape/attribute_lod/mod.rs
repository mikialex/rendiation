use std::{hash::Hash, sync::Arc};

use crate::*;

mod lod_convert;
pub use lod_convert::*;

mod draw_cmd;
pub use draw_cmd::*;
use parking_lot::Mutex;

pub fn use_attribute_lod_mesh_indirect_renderer(
  cx: &mut QueryGPUHookCx,
  init_config: &IndirectAttributeMeshInitConfig,
  merge_with_vertex_allocator: bool,
  force_midc_downgrade: bool,
  mesh_input: UseResult<AttributesMeshDataChangeInput>,
  node_info: Option<Box<dyn IndirectNodeRenderImpl>>,
  sm_world_aabb_info: Option<Box<dyn DrawUnitWorldBoundingProvider>>,
  current_lod_camera: CurrentLODCameraControl,
) -> Option<AttributeLODMeshIndirectRenderer> {
  let AttributeMeshLODConvertResult {
    processed_meshes,
    lod_metadata,
  } = process_attribute_mesh_lod(cx, mesh_input);

  let internal = use_attribute_mesh_indirect_renderer(
    cx,
    init_config,
    merge_with_vertex_allocator,
    force_midc_downgrade,
    processed_meshes,
  );

  let (lod_levels, allocation_info) = use_range_allocated_device_buffers::<LODLevelInfo>(
    cx,
    "attribute lod mesh level infos",
    128,
    u32::MAX,
    lod_metadata,
  );
  let range_change =
    allocation_info.map(|allocation_info| allocation_info.allocation_changes.clone());

  let (cx, level_meta) =
    cx.use_storage_buffer::<Vec2<u32>>("attribute lod mesh levels range info", 128, u32::MAX);
  range_change.update_storage_array(cx, level_meta, 0);

  level_meta.use_max_item_count_by_db_entity::<AttributesMeshEntity>(cx);
  level_meta.use_update(cx);

  cx.when_render(|| {
    //
    AttributeLODMeshIndirectRenderer {
      level_meta: level_meta.get_gpu_buffer(),
      lod_levels,
      sm_node_info: node_info.unwrap(),
      current_lod_camera,
      internal: internal.unwrap(),
      sm_world_aabb_info: sm_world_aabb_info.unwrap(),
    }
  })
}

pub struct AttributeLODMeshIndirectRenderer {
  level_meta: AbstractReadonlyStorageBuffer<[Vec2<u32>]>,
  lod_levels: AbstractReadonlyStorageBuffer<[LODLevelInfo]>,
  sm_node_info: Box<dyn IndirectNodeRenderImpl>,
  sm_world_aabb_info: Box<dyn DrawUnitWorldBoundingProvider>,
  current_lod_camera: CurrentLODCameraControl,
  pub internal: AttributeMeshIndirectRenderer,
}

impl DrawCommandBuilderCreator for AttributeLODMeshIndirectRenderer {
  fn make_draw_command_builder(&self, id: RawEntityHandle) -> Option<DrawCommandBuilder> {
    let (internal, is_indexed) = self.internal.make_draw_command_builder_impl(id)?;

    if is_indexed {
      let creator = AttributeLODMeshIndirectDrawCreator {
        internal,
        level_meta: self.level_meta.clone(),
        lod_levels: self.lod_levels.clone(),
        sm_node_info: self.sm_node_info.make_component_indirect().unwrap().clone(),
        sm_world_aabb_info: self.sm_world_aabb_info.clone(),
        lod_camera_info: self
          .current_lod_camera
          .get()
          .expect("active_lod_camera not set"),
      };

      DrawCommandBuilder::Indexed(Box::new(creator))
    } else {
      DrawCommandBuilder::NoneIndexed(Box::new(internal))
    }
    .into()
  }
}

impl IndirectDrawProviderCreator for AttributeLODMeshIndirectRenderer {
  fn get_impl_distinguish_key_by_impl_select_id(&self, id: RawEntityHandle) -> Option<u64> {
    let id = unsafe { EntityHandle::from_raw(id) };
    let mesh_id = self.internal.std_to_mesh.get(id)?;
    let indices_ty = self.internal.indices_ty.access(mesh_id.raw_handle_ref());

    fast_hash_scope(|hasher| {
      self.type_id().hash(hasher);
      indices_ty.hash(hasher);
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
      self.internal.used_in_midc_downgrade,
    )
    .into()
  }
}

impl IndirectModelShapeRenderImpl for AttributeLODMeshIndirectRenderer {
  fn make_component_indirect(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    self.internal.make_component_indirect(any_idx)
  }

  fn get_index_storage_buffer(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Option<IndicesBufferInfo>> {
    self.internal.get_index_storage_buffer(any_idx)
  }

  fn hash_shader_group_key(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    self.internal.hash_shader_group_key(any_idx, hasher)
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

#[derive(Default, Clone)]
pub struct CurrentLODCameraControl {
  current_view: Arc<Mutex<Option<LODCameraInfo>>>,
}

impl CurrentLODCameraControl {
  pub fn set(&self, camera: Option<LODCameraInfo>) {
    *self.current_view.lock() = camera;
  }

  pub fn get(&self) -> Option<LODCameraInfo> {
    self.current_view.lock().as_ref().cloned()
  }
}
