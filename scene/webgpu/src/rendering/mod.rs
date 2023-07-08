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
pub mod blur;
pub use blur::*;
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
pub mod lighting;
pub use lighting::*;

use crate::*;

pub struct SceneRenderResourceGroup<'a> {
  pub scene: &'a SceneCoreImpl,
  pub resources: &'a ContentGPUSystem,
  pub scene_resources: &'a SceneGPUSystem,
  pub node_derives: &'a SceneNodeDeriveSystem,
}

pub fn default_dispatcher(pass: &FrameRenderPass) -> DefaultPassDispatcher {
  DefaultPassDispatcher {
    formats: pass.ctx.pass.formats().clone(),
    pass_info: pass.pass_info.clone(),
    auto_write: true,
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

impl<'a> WebGPUScenePipelineHelper for SceneRenderResourceGroup<'a> {
  fn by_main_camera<T>(&self, inner: T) -> CameraRef<T> {
    CameraRef {
      camera: self.scene.active_camera.as_ref().unwrap(),
      inner,
    }
  }

  fn by_main_camera_and_self<T>(&self, inner: T) -> CameraSceneRef<T> {
    CameraSceneRef {
      camera: self.scene.active_camera.as_ref().unwrap(),
      scene: self,
      inner,
    }
  }
}

impl<'a, T: PassContentWithCamera> PassContent for CameraRef<'a, T> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    self.inner.render(pass, self.camera);
  }
}

pub trait PassContentWithCamera {
  fn render(&mut self, pass: &mut FrameRenderPass, camera: &SceneCamera);
}

pub trait PassContentWithSceneAndCamera {
  fn render(
    &mut self,
    pass: &mut FrameRenderPass,
    scene: &SceneRenderResourceGroup,
    camera: &SceneCamera,
  );
}

pub struct CameraSceneRef<'a, T> {
  pub camera: &'a SceneCamera,
  pub scene: &'a SceneRenderResourceGroup<'a>,
  pub inner: T,
}

impl<'a, T> PassContent for CameraSceneRef<'a, T>
where
  T: PassContentWithSceneAndCamera,
{
  fn render(&mut self, pass: &mut FrameRenderPass) {
    self.inner.render(pass, self.scene, self.camera);
  }
}
