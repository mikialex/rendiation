#![feature(allocator_api)]

use std::{alloc::Allocator, any::TypeId, hash::Hash, sync::Arc};

use database::*;
use fast_hash_collection::*;
use rendiation_algebra::*;
use rendiation_device_parallel_compute::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_shader_api::*;
use rendiation_shader_library::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;
use serde::*;

mod lod_convert;
pub use lod_convert::*;

mod draw_cmd;
pub use draw_cmd::*;
use parking_lot::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeLODConfig {
  /// the current conversion mode
  pub lod_conversion_mode: LODConversionMode,

  /// meshes with less triangles than this are not converted, the benefit is too low
  pub min_lod_triangle_count: usize,

  pub error_double_mode_config: ErrorDoublingConfig,
}

impl Default for AttributeLODConfig {
  fn default() -> Self {
    Self {
      lod_conversion_mode: LODConversionMode::HalfCount,
      min_lod_triangle_count: 32,
      error_double_mode_config: Default::default(),
    }
  }
}

pub fn use_attribute_lod_mesh_indirect_renderer(
  cx: &mut QueryGPUHookCx,
  init_config: &IndirectAttributeMeshInitConfig,
  lod_init_config: &AttributeLODConfig,
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
  } = process_attribute_mesh_lod(cx, mesh_input, lod_init_config);

  let internal = use_attribute_mesh_indirect_renderer(
    cx,
    init_config,
    merge_with_vertex_allocator,
    force_midc_downgrade,
    processed_meshes,
  );

  let (lod_metadata, lod_metadata_) = lod_metadata.fork();

  let (lod_levels, allocation_info) = use_range_allocated_device_buffers::<LODLevelInfo>(
    cx,
    "attribute lod mesh level infos",
    128,
    u32::MAX,
    lod_metadata,
  );
  let range_change =
    allocation_info.map(|allocation_info| allocation_info.allocation_changes.clone());

  // the level meta contains (levels offset, level count, root level count),
  // the root level count is used by the host access to draw the origin mesh without lod
  let (cx, level_meta) = cx.use_storage_buffer_with_host_backup::<Vec4<u32>>(
    "attribute lod mesh levels range info",
    128,
    u32::MAX,
  );
  range_change.update_storage_array_with_host(cx, level_meta, 0);

  let root_count_changes = lod_metadata_.map(|changes| {
    let mut root_counts = Vec::new();
    for (k, levels) in changes.iter_update_or_insert() {
      let root_count = levels.first().map(|l| l.count).unwrap_or(0);
      root_counts.push((k, root_count));
    }
    Arc::new(LinearBatchChanges {
      removed: changes.removed.clone(),
      update_or_insert: root_counts,
    })
  });
  root_count_changes.update_storage_array_with_host(
    cx,
    level_meta,
    std::mem::offset_of!(Vec4<u32>, z),
  );

  level_meta.use_max_item_count_by_db_entity::<AttributesMeshEntity>(cx);
  level_meta.use_update(cx);

  cx.when_render(|| {
    //
    AttributeLODMeshIndirectRenderer {
      level_meta: level_meta.get_gpu_buffer(),
      level_meta_host: level_meta.buffer.make_read_holder(),
      lod_levels,
      sm_node_info: node_info.unwrap(),
      current_lod_camera,
      internal: internal.unwrap(),
      sm_world_aabb_info: sm_world_aabb_info.unwrap(),
    }
  })
}

pub struct AttributeLODMeshIndirectRenderer {
  level_meta: AbstractReadonlyStorageBuffer<[Vec4<u32>]>,
  level_meta_host: LockReadGuardHolder<SparseStorageBufferWithHostRaw<Vec4<u32>>>,
  lod_levels: AbstractReadonlyStorageBuffer<[LODLevelInfo]>,
  sm_node_info: Box<dyn IndirectNodeRenderImpl>,
  sm_world_aabb_info: Box<dyn DrawUnitWorldBoundingProvider>,
  current_lod_camera: CurrentLODCameraControl,
  pub internal: AttributeMeshIndirectRenderer,
}

impl IndirectModelShapeRendererResolver for AttributeLODMeshIndirectRenderer {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelShapeRendererBox<'a>, SceneModelRenderDataError>> {
    let internal = self.internal.resolve_attribute_mesh(idx)?;
    Some(internal.map(|internal| {
      let resolved = ResolvedAttributeLODMeshIndirectRenderer {
        renderer: self,
        internal,
      };
      Box::new_in(resolved, alloc) as _
    }))
  }
}

struct ResolvedAttributeLODMeshIndirectRenderer<'a> {
  renderer: &'a AttributeLODMeshIndirectRenderer,
  internal: ResolvedAttributeMeshIndirectRenderer<'a>,
}

impl ResolvedIndirectModelShapeRenderer for ResolvedAttributeLODMeshIndirectRenderer<'_> {
  fn resolver_type_id(&self) -> TypeId {
    TypeId::of::<AttributeLODMeshIndirectRenderer>()
  }

  fn hash_shader_group_key(&self, hasher: &mut PipelineHasher) {
    self.internal.hash_shader_group_key(hasher)
  }

  fn impl_distinguish_key(&self) -> u64 {
    fast_hash_scope(|hasher| {
      self.resolver_type_id().hash(hasher);
      // index type not matters
      self.internal.indices_ty.is_some().hash(hasher);
    })
  }

  fn use_create_or_update_indirect_draw_providers(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    list: &DeviceDrawList,
  ) -> Result<Vec<Box<dyn IndirectDrawProvider>>, SceneModelRenderDataError> {
    Ok(use_and_create_default_indirect_draw_provider(
      list,
      self.make_draw_command_builder()?,
      cx,
      self.renderer.internal.used_in_midc_downgrade,
    ))
  }

  fn make_draw_command_builder(&self) -> Result<DrawCommandBuilder, SceneModelRenderDataError> {
    let err = SceneModelRenderDataError;
    let (internal, is_indexed) = self.internal.make_draw_command_builder_impl();
    let renderer = self.renderer;

    let builder = if is_indexed {
      let creator = AttributeLODMeshIndirectDrawCreator {
        internal,
        level_meta: renderer.level_meta.clone(),
        level_meta_host: renderer.level_meta_host.clone(),
        lod_levels: renderer.lod_levels.clone(),
        sm_node_info: renderer
          .sm_node_info
          .make_component_indirect()
          .ok_or(err("lod node info"))?,
        sm_world_aabb_info: renderer.sm_world_aabb_info.clone(),
        lod_camera_info: renderer
          .current_lod_camera
          .get()
          .expect("active_lod_camera not set"),
      };

      DrawCommandBuilder::Indexed(Box::new(creator))
    } else {
      DrawCommandBuilder::NoneIndexed(Box::new(internal))
    };
    Ok(builder)
  }

  fn make_component_indirect<'a>(&'a self, alloc: &'a dyn Allocator) -> RenderComponentBox<'a> {
    self.internal.make_component_indirect(alloc)
  }

  fn get_index_storage_buffer(&self) -> Option<IndicesBufferInfo> {
    self.internal.get_index_storage_buffer()
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
