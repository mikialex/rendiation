use std::{sync::Arc, task::Context};

use database::*;
use parking_lot::RwLock;
use reactive::{
  BoxedDynReactiveQuery, Query, QueryCompute, ReactiveGeneralQuery, ReactiveQuery, ReactiveQueryExt,
};
use rendiation_device_parallel_compute::DeviceParallelComputeCtx;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod scene_integration;
pub use scene_integration::*;
mod lod;
use lod::*;
mod draw_access;
use draw_access::*;
mod draw_prepare;
use draw_prepare::*;
use rendiation_mesh_lod_graph::*;
use rendiation_webgpu_reactive_utils::*;

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

pub fn use_mesh_lod_graph_renderer(
  qcx: &mut impl QueryGPUHookCx,
) -> Option<MeshLODGraphRendererShared> {
  qcx.use_gpu_general_query(|gpu| MeshLODGraphRendererSystem {
    source: global_watch().watch::<LODGraphData>().into_boxed(),
    renderer: Arc::new(RwLock::new(MeshLODGraphRenderer::new(gpu))),
    gpu: gpu.clone(),
  })
}

pub struct MeshLODGraphRendererSystem {
  source:
    BoxedDynReactiveQuery<EntityHandle<LODGraphMeshEntity>, Option<ExternalRefPtr<MeshLODGraph>>>,
  renderer: MeshLODGraphRendererShared,
  gpu: GPU,
}

pub type MeshLODGraphRendererShared = Arc<RwLock<MeshLODGraphRenderer>>;

impl ReactiveGeneralQuery for MeshLODGraphRendererSystem {
  type Output = MeshLODGraphRendererShared;
  fn poll_query(&mut self, cx: &mut Context) -> MeshLODGraphRendererShared {
    let mut renderer = self.renderer.write();
    let (change, _) = self.source.describe(cx).resolve_kept();

    for (key, change) in change.iter_key_value() {
      match change {
        reactive::ValueChange::Delta(mesh, previous) => {
          if let Some(Some(_)) = previous {
            renderer.remove_mesh(key);
          }
          if let Some(mesh) = mesh {
            renderer.add_mesh(key, &mesh, &self.gpu);
          }
        }
        reactive::ValueChange::Remove(_) => renderer.remove_mesh(key),
      }
    }
    self.renderer.clone()
  }
}

pub struct MeshLODGraphRenderer {
  pub meshlet_metadata_host: Vec<MeshletMetaData>,
  pub meshlet_metadata: StorageBufferRangeAllocatePool<MeshletMetaData>,
  pub scene_model_meshlet_range_host: Vec<Vec2<u32>>,
  pub scene_model_meshlet_range: StorageBufferReadonlyDataView<[Vec2<u32>]>,
  pub position_buffer: StorageBufferRangeAllocatePool<u32>,
  pub index_buffer: StorageBufferRangeAllocatePool<u32>,
}

fn compute_lod_graph_render_data(graph: &MeshLODGraph) {
  //
}

impl MeshLODGraphRenderer {
  pub fn new(gpu: &GPU) -> Self {
    todo!()
  }

  pub fn add_mesh(
    &mut self,
    key: EntityHandle<LODGraphMeshEntity>,
    mesh: &MeshLODGraph,
    gpu: &GPU,
  ) {
    // self.index_buffer.allocate_values(v, relocation_handler)
    //
  }
  pub fn remove_mesh(&mut self, key: EntityHandle<LODGraphMeshEntity>) {
    todo!()
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
      StorageBufferReadonlyDataView::try_from_raw(self.meshlet_metadata.raw_gpu().clone()).unwrap();

    let expander = MeshLODExpander {
      meshlet_metadata,
      scene_model_meshlet_range: self.scene_model_meshlet_range.clone(),
      lod_decider,
    };

    Box::new(expander.expand(batch, scene_model_matrix, cx, max_meshlet_count))
  }

  pub fn create_mesh_accessor(&self) -> Box<dyn RenderComponent> {
    let meshlet_metadata =
      StorageBufferReadonlyDataView::try_from_raw(self.meshlet_metadata.raw_gpu().clone()).unwrap();

    let position_buffer =
      StorageBufferReadonlyDataView::try_from_raw(self.position_buffer.raw_gpu().clone()).unwrap();

    let index_buffer =
      StorageBufferReadonlyDataView::try_from_raw(self.index_buffer.raw_gpu().clone()).unwrap();

    Box::new(MeshletGPURenderData {
      meshlet_metadata,
      position_buffer,
      index_buffer,
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
