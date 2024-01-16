#![allow(dead_code)]
#![allow(unused_variables)]

use std::marker::PhantomData;

use rendiation_algebra::*;
use rendiation_scene_gpu_base::{SceneRasterRenderingAdaptor, SceneRenderingAdaptorBase};
use rendiation_webgpu::util::DrawIndexedIndirect;

mod lod_selection;
use lod_selection::*;

mod frustum_culling;
use frustum_culling::*;

struct StorageBuffer<T>(T);
struct StorageArrayHandle<T>(PhantomData<T>);
type MaterialA = usize;
type MaterialB = usize;

pub struct DeviceSceneRepresentation<T> {
  adaptor: T,
  models: StorageBuffer<ShaderSceneModelInfo>,
  nodes: StorageBuffer<ShaderNodeInfo>,
  meshes: StorageBuffer<DrawIndexedIndirect>,

  material_a: StorageBuffer<MaterialA>,
  material_b: StorageBuffer<MaterialB>,

  lod_mesh: StorageBuffer<LODMetaData>,
  common_mesh: StorageBuffer<DrawIndexedIndirect>,
}

impl<T: SceneRasterRenderingAdaptor> SceneRenderingAdaptorBase for DeviceSceneRepresentation<T> {
  fn build(scene: rendiation_scene_core::Scene) -> Self {
    todo!()
  }

  fn poll_update(&mut self, cx: &mut std::task::Context) {
    todo!()
  }
}

impl<T: SceneRasterRenderingAdaptor> SceneRasterRenderingAdaptor for DeviceSceneRepresentation<T> {
  type DrawTask = usize;

  fn create_task(camera: &rendiation_scene_core::SceneCamera) -> Self::DrawTask {
    todo!()
  }

  fn render_task_on_frame(
    &self,
    ctx: &mut rendiation_webgpu::FrameCtx,
    task: Self::DrawTask,
    target: &rendiation_webgpu::Attachment,
  ) {
    todo!()
  }
}

// maintained by cpu side
struct ShaderSceneModelInfo {
  pub material_idx: u32,
  pub material_type_idx: u32,
  pub mesh_idx: u32,
  pub mesh_type_idx: u32,
  pub node_idx: StorageArrayHandle<ShaderNodeInfo>,
  pub world_aabb: ShaderAABB,
}

// maintained by cpu side
pub struct ShaderNodeInfo {
  pub world_mat: Mat4<f32>,
  pub filter_flags: u32,
}

// not retained
pub struct DrawCommandBuffer {
  model_idx: StorageArrayHandle<ShaderSceneModelInfo>,
}

// pub fn update_gpu_storage<T>(
//   buffer: GPUStorageBuffer<[T]>,
//   source: impl ReactiveCollection<usize, T>,
// ) {
//   //
// }
