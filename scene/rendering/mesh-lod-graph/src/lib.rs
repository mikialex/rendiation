use std::sync::Arc;

use database::*;
use parking_lot::RwLock;
use rendiation_device_parallel_compute::DeviceParallelComputeCtx;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod scene_integration;
use rendiation_webgpu_midc_downgrade::*;
pub use scene_integration::*;
mod lod;
use lod::*;
mod draw_access;
use draw_access::*;
mod draw_prepare;
use draw_prepare::*;
pub use rendiation_mesh_lod_graph::*;
use rendiation_webgpu_hook_utils::*;

declare_entity!(LODGraphMeshEntity);
declare_component!(
  LODGraphData,
  LODGraphMeshEntity,
  Option<ExternalRefPtr<MeshLODGraph>>
);

pub fn register_mesh_lod_graph_data_model() {
  global_database()
    .declare_entity::<LODGraphMeshEntity>()
    .declare_component::<LODGraphData>();
}

pub fn use_mesh_lod_graph_renderer(cx: &mut QueryGPUHookCx) -> MeshLODGraphRendererShared {
  let (cx, renderer) = cx.use_gpu_init(|gpu| Arc::new(RwLock::new(MeshLODGraphRenderer::new(gpu))));

  if let Some(change) = cx.use_query_change::<LODGraphData>().if_ready() {
    renderer.write().batch_update(change.mark_entity_type());
  }

  renderer.clone()
}

pub type MeshLODGraphRendererShared = Arc<RwLock<MeshLODGraphRenderer>>;

pub struct MeshLODGraphRenderer {
  pub scene_model_meshlet_index_vertex_offset: Vec<Vec2<u32>>,
  pub meshlet_metadata: StorageBufferRangeAllocatePool<MeshletMetaData>,
  pub scene_model_meshlet_range: CommonStorageBufferImplWithHostBackup<Vec2<u32>>,
  pub position_buffer: StorageBufferRangeAllocatePool<u32>,
  pub index_buffer: StorageBufferRangeAllocatePool<u32>,
  pub enable_midc_downgrade: bool,
}

impl MeshLODGraphRenderer {
  pub fn new(gpu: &GPU) -> Self {
    let max_indices_count = 1_000_000_u32;
    let indices = StorageBufferReadonlyDataView::<[u32]>::create_by_with_extra_usage(
      &gpu.device,
      "mesh lod graph indices pool".into(),
      ZeroedArrayByArrayLength(max_indices_count as usize).into(),
      BufferUsages::INDEX,
    );

    let indices = create_growable_buffer(gpu, indices, max_indices_count);
    let index_buffer = GPURangeAllocateMaintainer::new(gpu, indices, max_indices_count);

    Self {
      scene_model_meshlet_index_vertex_offset: vec![Default::default(); 100],
      meshlet_metadata: create_storage_buffer_range_allocate_pool(
        gpu,
        "mesh lod graph meshlet metadata pool",
        10000,
        10000,
      ),
      scene_model_meshlet_range: create_common_storage_buffer_with_host_backup_container(
        100, 100, gpu,
      ),
      position_buffer: create_storage_buffer_range_allocate_pool(
        gpu,
        "mesh lod graph meshlet vertex pool: position",
        1_000_000,
        1_000_000,
      ),
      index_buffer,
      enable_midc_downgrade: require_midc_downgrade(&gpu.info),
    }
  }

  pub fn batch_update(
    &mut self,
    change: impl Query<
      Key = EntityHandle<LODGraphMeshEntity>,
      Value = ValueChange<Option<ExternalRefPtr<MeshLODGraph>>>,
    >,
  ) {
    for (key, change) in change.iter_key_value() {
      match change {
        ValueChange::Delta(mesh, previous) => {
          if let Some(Some(_)) = previous {
            self.remove_mesh(key);
          }
          if let Some(mesh) = mesh {
            self.add_mesh(key, &mesh);
          }
        }
        ValueChange::Remove(_) => self.remove_mesh(key),
      }
    }
  }

  // todo, support other vertex channel
  pub fn add_mesh(&mut self, key: EntityHandle<LODGraphMeshEntity>, mesh: &MeshLODGraph) {
    let mut grow_assert = |_| unreachable!("grow is not expected");

    let mut meshlet_gpu_data: Vec<_> = Default::default();

    let mut index: Vec<_> = Default::default();
    let mut position: Vec<_> = Default::default();

    for level in &mesh.levels {
      index.extend(level.mesh.indices.clone());
      position.extend(level.mesh.vertices.iter().map(|v| v.position));
    }

    let base_index_offset = self
      .index_buffer
      .allocate_values(&index, &mut grow_assert)
      .unwrap();

    let base_position_offset = self
      .position_buffer
      .allocate_values(cast_slice(&position), &mut grow_assert)
      .unwrap();

    self.scene_model_meshlet_index_vertex_offset[key.alloc_index() as usize] =
      vec2(base_index_offset, base_position_offset);

    self.scene_model_meshlet_index_vertex_offset[key.alloc_index() as usize] = vec2(0, 0);

    let mut base_index_offset = base_index_offset;
    let mut base_position_offset = base_position_offset;

    for (level_index, level) in mesh.levels.iter().enumerate() {
      base_index_offset += level.mesh.indices.len() as u32;
      base_position_offset += level.mesh.vertices.len() as u32;

      let meshlet_gpu_data_level = level.meshlets.iter().map(|meshlet| MeshletMetaData {
        index_offset: base_index_offset + meshlet.index_range.offset,
        index_count: meshlet.index_range.size,
        position_offset: base_position_offset,
        bounds: {
          let self_group = level.groups[meshlet.group_index as usize];
          let self_lod = LODBound::new_from_group(&self_group);
          let parent_lod = if level_index == 0 {
            LODBound::new(
              0.,
              self_group.union_meshlet_bounding_among_meshlet_in_their_parent_group,
            )
          } else {
            let parent_level = &mesh.levels[level_index - 1];
            let parent_group = parent_level.groups[meshlet.group_index_in_previous_level as usize];
            LODBound::new_from_group(&parent_group)
          };
          LODBoundPair::new(self_lod, parent_lod)
        },
        ..Default::default()
      });

      meshlet_gpu_data.extend(meshlet_gpu_data_level);
    }

    let meshlet_offset = self
      .meshlet_metadata
      .allocate_values(&meshlet_gpu_data, &mut grow_assert)
      .unwrap();

    let mesh_range = vec2(
      meshlet_offset,
      meshlet_offset + meshlet_gpu_data.len() as u32,
    );
    self
      .scene_model_meshlet_range
      .set_value(key.alloc_index(), mesh_range)
      .unwrap();
  }

  pub fn remove_mesh(&mut self, key: EntityHandle<LODGraphMeshEntity>) {
    let mesh_range = *self
      .scene_model_meshlet_range
      .get(key.alloc_index())
      .unwrap();
    let index_vertex_offset = *self
      .scene_model_meshlet_index_vertex_offset
      .get(key.alloc_index() as usize)
      .unwrap();

    self
      .scene_model_meshlet_range
      .set_value(key.alloc_index(), vec2(0, 0))
      .unwrap();
    self.scene_model_meshlet_index_vertex_offset[key.alloc_index() as usize] = vec2(0, 0);

    self.meshlet_metadata.deallocate(mesh_range.x);
    self.index_buffer.deallocate(index_vertex_offset.x);
    self.position_buffer.deallocate(index_vertex_offset.y);
  }

  pub fn prepare_draw(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    cx: &mut DeviceParallelComputeCtx,
    lod_decider: UniformBufferDataView<LODDecider>,
    scene_model_matrix: &dyn DrawUnitWorldTransformProvider,
    max_meshlet_count: u32,
  ) -> Box<dyn IndirectDrawProvider> {
    let meshlet_metadata =
      StorageBufferReadonlyDataView::try_from_raw(self.meshlet_metadata.gpu().gpu.clone()).unwrap();

    let expander = MeshLODExpander {
      meshlet_metadata,
      scene_model_meshlet_range: self.scene_model_meshlet_range.gpu().clone(),
      lod_decider,
    };

    let batch = expander.expand(batch, scene_model_matrix, cx, max_meshlet_count);
    into_maybe_downgrade_batch_assume_standard_midc_style(batch, cx)
  }

  pub fn create_mesh_accessor(&self) -> Box<dyn RenderComponent> {
    let meshlet_metadata =
      StorageBufferReadonlyDataView::try_from_raw(self.meshlet_metadata.gpu().gpu.clone()).unwrap();

    let position_buffer =
      StorageBufferReadonlyDataView::try_from_raw(self.position_buffer.gpu().gpu.clone()).unwrap();

    let index_buffer =
      StorageBufferReadonlyDataView::try_from_raw(self.index_buffer.gpu().gpu.clone()).unwrap();

    let draw_data = MeshletGPURenderData {
      meshlet_metadata,
      position_buffer,
      index_buffer,
    };

    Box::new(MidcDowngradeWrapperForIndirectMeshSystem {
      index: draw_data.index_buffer.clone().into(),
      mesh_system: draw_data,
      enable_downgrade: self.enable_midc_downgrade,
    })
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, Copy, ShaderStruct, PartialEq)]
pub struct MeshletMetaData {
  pub index_offset: u32,
  pub index_count: u32,
  pub position_offset: u32,
  pub bounds: LODBoundPair,
}

impl Default for MeshletMetaData {
  fn default() -> Self {
    Self {
      index_offset: u32::MAX,
      position_offset: u32::MAX,
      ..Zeroable::zeroed()
    }
  }
}
