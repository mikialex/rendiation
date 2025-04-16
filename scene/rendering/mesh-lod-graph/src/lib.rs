use rendiation_device_parallel_compute::DeviceParallelComputeCtx;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod lod;
use lod::*;
mod draw_access;
use draw_access::*;
mod draw_prepare;
use draw_prepare::*;

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

pub struct MeshLODGraphRenderer {
  pub meshlet_metadata: StorageBufferReadonlyDataView<[MeshletMetaData]>,
  pub scene_model_meshlet_range: StorageBufferReadonlyDataView<[Vec2<u32>]>,
  pub position_buffer: StorageBufferReadonlyDataView<[u32]>,
  pub meshlet_buffer: StorageBufferReadonlyDataView<[u32]>,
  pub index_buffer: StorageBufferReadonlyDataView<[u32]>,
}

impl MeshLODGraphRenderer {
  pub fn prepare_draw(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    cx: &mut DeviceParallelComputeCtx,
    lod_decider: LODDecider,
    scene_model_matrix: &dyn SceneModelWorldMatrixProvider,
    max_meshlet_count: u32,
  ) -> Box<dyn IndirectDrawProvider> {
    let lod_decider = create_uniform(lod_decider, &cx.gpu.device);

    let expander = MeshLODExpander {
      meshlet_metadata: self.meshlet_metadata.clone(),
      scene_model_meshlet_range: self.scene_model_meshlet_range.clone(),
      lod_decider,
    };

    Box::new(expander.expand(batch, scene_model_matrix, cx, max_meshlet_count))
  }

  pub fn create_mesh_accessor(&self) -> Box<dyn RenderComponent> {
    Box::new(MeshletGPURenderData {
      meshlet_metadata: self.meshlet_metadata.clone(),
      position_buffer: self.position_buffer.clone(),
      index_buffer: self.index_buffer.clone(),
    })
  }
}
