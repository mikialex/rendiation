mod background;
pub use background::*;
mod debug_channels;
pub use debug_channels::*;
mod shadow;
pub use shadow::*;
mod content;
pub use content::*;
mod lighting;
pub use lighting::*;

use crate::*;

pub struct SceneRenderResourceGroup<'a> {
  pub scene: &'a SceneCoreImpl,
  pub resources: &'a ContentGPUSystem,
  pub scene_resources: &'a SceneGPUSystem,
  pub node_derives: &'a SceneNodeDeriveSystem,
}

impl<'a> SceneRenderResourceGroup<'a> {
  pub fn extend_bindless_resource_provider<T>(
    &'a self,
    dispatcher: &'a T,
  ) -> BindlessResourceProvider<'a, T> {
    BindlessResourceProvider {
      base: dispatcher,
      texture_system: &self.resources.bindable_ctx.binding_sys,
    }
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
