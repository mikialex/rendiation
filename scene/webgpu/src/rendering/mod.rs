mod background;
pub use background::*;

mod content;
pub use content::*;

// mod shadow;
// pub use shadow::*;
// mod lighting;
// pub use lighting::*;
use crate::*;

pub struct SceneRenderResourceGroup<'a> {
  pub scene: &'a SceneCoreImpl,
  pub texture_gpu: &'a GPUTextureBindingSystem,

  pub resources: &'a SceneShareContentGPUResource,
  pub scene_resources: &'a SceneGPUResource,
  pub node_derives: &'a SceneNodeDeriveSystem,
}

impl<'a> SceneRenderResourceGroup<'a> {
  pub fn with_scene_resource_ctx<T>(
    &'a self,
    dispatcher: &'a T,
  ) -> BindlessResourceProvider<'a, T> {
    BindlessResourceProvider {
      base: dispatcher,
      texture_system: &self.texture_gpu,
    }
  }
}

pub struct CameraRef<'a, T> {
  camera: &'a SceneRenderCameraCtx<'a>,
  inner: T,
}

impl<'a, T> CameraRef<'a, T> {
  pub fn with(camera: &'a SceneCamera, inner: T) -> Self {
    CameraRef {
      camera: todo!(),
      inner,
    }
  }
}

pub trait WebGPUScenePipelineHelper {
  fn by_main_camera<T>(&self, inner: T) -> CameraRef<T>;
  fn by_main_camera_and_self<T>(&self, inner: T) -> CameraSceneRef<T>;
}

impl<'a> WebGPUScenePipelineHelper for SceneRenderResourceGroup<'a> {
  fn by_main_camera<T>(&self, inner: T) -> CameraRef<T> {
    CameraRef {
      camera: todo!(),
      inner,
    }
  }

  fn by_main_camera_and_self<T>(&self, inner: T) -> CameraSceneRef<T> {
    CameraSceneRef {
      camera: todo!(),
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
  fn render(&mut self, pass: &mut FrameRenderPass, camera: &SceneRenderCameraCtx);
}

pub trait PassContentWithSceneAndCamera {
  fn render(
    &mut self,
    pass: &mut FrameRenderPass,
    scene: &SceneRenderResourceGroup,
    camera: &SceneRenderCameraCtx,
  );
}

pub struct CameraSceneRef<'a, T> {
  pub camera: &'a SceneRenderCameraCtx<'a>,
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
