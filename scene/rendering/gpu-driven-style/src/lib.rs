#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use std::marker::PhantomData;

use __core::task::Context;
use rendiation_algebra::*;
use rendiation_mesh_gpu_system::{DrawIndexedIndirect, GPUBindlessMeshSystem};
use rendiation_scene_core::*;
use rendiation_scene_gpu_base::{SceneRasterRenderingAdaptor, SceneRenderingAdaptorBase};
use rendiation_shader_api::*;
use rendiation_texture_gpu_system::AbstractIndirectGPUTextureSystem;

mod lod_selection;
use lod_selection::*;

mod frustum_culling;
use frustum_culling::*;

mod occlusion_culling;
use occlusion_culling::*;
use rendiation_webgpu::{Attachment, FrameCtx};

struct StorageBuffer<T: ?Sized>(Box<T>);
struct StorageArrayHandle<T>(PhantomData<T>, u32);
type MaterialA = usize;
type MaterialB = usize;

pub struct DeviceSceneRepresentation<T> {
  adaptor: T,
  models: StorageBuffer<[ShaderSceneModelInfo]>,
  nodes: StorageBuffer<[ShaderNodeInfo]>,
  meshes: StorageBuffer<[DrawIndexedIndirect]>,

  material_a: StorageBuffer<[MaterialA]>,
  material_b: StorageBuffer<[MaterialB]>,

  lod_mesh: StorageBuffer<[LODMetaData]>,
  common_mesh: StorageBuffer<[DrawIndexedIndirect]>,

  mesh: GPUBindlessMeshSystem,
  textures: Box<dyn AbstractIndirectGPUTextureSystem>,
}

impl<T: SceneRasterRenderingAdaptor> SceneRenderingAdaptorBase for DeviceSceneRepresentation<T> {
  fn build(scene: Scene) -> Self {
    todo!()
  }

  fn poll_update(&mut self, cx: &mut Context) {
    todo!()
  }
}

impl<T: SceneRasterRenderingAdaptor> SceneRasterRenderingAdaptor for DeviceSceneRepresentation<T> {
  type DrawTask = usize;

  fn create_task(camera: &SceneCamera) -> Self::DrawTask {
    todo!()
  }

  fn render_task_on_frame(&self, ctx: &mut FrameCtx, task: Self::DrawTask, target: &Attachment) {
    todo!()
  }
}

// maintained by cpu side
#[repr(C)]
// #[std430_layout]
// #[derive(Clone, Copy, ShaderStruct, Debug)]
struct ShaderSceneModelInfo {
  pub material_idx: u32,
  pub material_type_idx: u32,
  pub mesh_idx: u32,
  pub mesh_type_idx: u32,
  pub node_idx: StorageArrayHandle<ShaderNodeInfo>,
  pub world_aabb: ShaderAABB,
}

// maintained by cpu side
#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug)]
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
