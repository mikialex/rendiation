pub mod forward;
pub use forward::*;
pub mod list;
pub use list::*;
pub mod copy_frame;
pub use copy_frame::*;
pub mod highlight;
pub use highlight::*;
pub mod background;
pub use background::*;
pub mod quad;
pub use quad::*;
pub mod framework;
pub use framework::*;
pub mod blur;
pub use blur::*;
pub mod defer;
pub use defer::*;
pub mod tonemap;
pub use tonemap::*;
pub mod debug_channels;
pub use debug_channels::*;
pub mod taa;
pub use taa::*;
pub mod ssao;
pub use ssao::*;
pub mod pass_base;
pub use pass_base::*;

use crate::*;

pub trait MeshDrawcallEmitter {
  fn draw(&self, ctx: &mut GPURenderPassCtx, group: MeshDrawGroup);
}

pub struct MeshDrawcallEmitterWrap<'a> {
  pub group: MeshDrawGroup,
  pub mesh: &'a dyn MeshDrawcallEmitter,
}

impl<'a> DrawcallEmitter for MeshDrawcallEmitterWrap<'a> {
  fn draw(&self, ctx: &mut GPURenderPassCtx) {
    self.mesh.draw(ctx, self.group)
  }
}

pub struct SceneRenderPass<'a, 'b, 'c> {
  pub ctx: GPURenderPassCtx<'a, 'b>,
  pub resources: &'c mut GPUResourceCache,
  pub pass_info: UniformBufferDataView<RenderPassGPUInfoData>,
}

impl<'a, 'b, 'c> SceneRenderPass<'a, 'b, 'c> {
  pub fn default_dispatcher(&self) -> DefaultPassDispatcher {
    DefaultPassDispatcher {
      formats: self.ctx.pass.formats().clone(),
      pass_info: self.pass_info.clone(),
      auto_write: true,
    }
  }
}

impl<'a, 'b, 'c> std::ops::Deref for SceneRenderPass<'a, 'b, 'c> {
  type Target = GPURenderPass<'a>;

  fn deref(&self) -> &Self::Target {
    &self.ctx.pass
  }
}

impl<'a, 'b, 'c> std::ops::DerefMut for SceneRenderPass<'a, 'b, 'c> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.ctx.pass
  }
}

pub struct CameraRef<'a, T> {
  camera: &'a SceneCamera,
  inner: T,
}

impl<'a, T> CameraRef<'a, T> {
  pub fn with(camera: &'a SceneCamera, inner: T) -> Self {
    CameraRef { camera, inner }
  }
}

pub trait WebGPUScenePipelineHelper {
  fn by_main_camera<T>(&self, inner: T) -> CameraRef<T>;
  fn by_main_camera_and_self<T>(&self, inner: T) -> CameraSceneRef<T>;
}

impl WebGPUScenePipelineHelper for SceneInner {
  fn by_main_camera<T>(&self, inner: T) -> CameraRef<T> {
    CameraRef {
      camera: self.active_camera.as_ref().unwrap(),
      inner,
    }
  }

  fn by_main_camera_and_self<T>(&self, inner: T) -> CameraSceneRef<T> {
    CameraSceneRef {
      camera: self.active_camera.as_ref().unwrap(),
      scene: self,
      inner,
    }
  }
}

impl<'a, T: PassContentWithCamera> PassContent for CameraRef<'a, T> {
  fn render(&mut self, pass: &mut SceneRenderPass) {
    self.inner.render(pass, self.camera);
  }
}

pub trait PassContentWithCamera {
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera);
}

pub trait PassContentWithSceneAndCamera {
  fn render(&mut self, pass: &mut SceneRenderPass, scene: &SceneInner, camera: &SceneCamera);
}

pub struct CameraSceneRef<'a, T> {
  pub camera: &'a SceneCamera,
  pub scene: &'a SceneInner,
  pub inner: T,
}

impl<'a, T> PassContent for CameraSceneRef<'a, T>
where
  T: PassContentWithSceneAndCamera,
{
  fn render(&mut self, pass: &mut SceneRenderPass) {
    self.inner.render(pass, self.scene, self.camera);
  }
}

pub trait RebuildAbleGPUCollectionBase {
  fn reset(&mut self);
  /// return count
  fn update_gpu(&mut self, gpu: &GPU) -> usize;
}
