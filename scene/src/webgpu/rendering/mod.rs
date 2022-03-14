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
pub mod utils;
use rendiation_webgpu::{BindingBuilder, GPURenderPass, GPU};
pub use utils::*;

pub mod framework;
pub use framework::*;

use crate::{GPUResourceCache, Scene, SceneCamera};

pub struct SceneRenderPass<'a, 'b> {
  pub pass: GPURenderPass<'a>,
  pub binding: BindingBuilder,
  pub resources: &'b mut GPUResourceCache,
}

impl<'a, 'b> std::ops::Deref for SceneRenderPass<'a, 'b> {
  type Target = GPURenderPass<'a>;

  fn deref(&self) -> &Self::Target {
    &self.pass
  }
}

impl<'a, 'b> std::ops::DerefMut for SceneRenderPass<'a, 'b> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.pass
  }
}

pub struct CameraRef<'a, T> {
  camera: &'a SceneCamera,
  inner: T,
}

impl Scene {
  pub fn by_main_camera<T>(&self, inner: T) -> CameraRef<T> {
    CameraRef {
      camera: self.active_camera.as_ref().unwrap(),
      inner,
    }
  }

  pub fn by_main_camera_and_self<T>(&self, inner: T) -> CameraSceneRef<T> {
    CameraSceneRef {
      camera: self.active_camera.as_ref().unwrap(),
      scene: self,
      inner,
    }
  }
}

impl<'a, T: PassContentWithCamera> PassContent for CameraRef<'a, T> {
  fn render(&mut self, gpu: &rendiation_webgpu::GPU, pass: &mut SceneRenderPass) {
    self.inner.render(gpu, pass, self.camera);
  }
}

pub trait PassContentWithCamera {
  fn render(&mut self, gpu: &GPU, pass: &mut SceneRenderPass, camera: &SceneCamera);
}

pub trait PassContentWithSceneAndCamera {
  fn render(&mut self, gpu: &GPU, pass: &mut SceneRenderPass, scene: &Scene, camera: &SceneCamera);
}

pub struct CameraSceneRef<'a, T> {
  camera: &'a SceneCamera,
  scene: &'a Scene,
  inner: T,
}

impl<'a, T: PassContentWithSceneAndCamera> PassContent for CameraSceneRef<'a, T> {
  fn render(&mut self, gpu: &rendiation_webgpu::GPU, pass: &mut SceneRenderPass) {
    self.inner.render(gpu, pass, self.scene, self.camera);
  }
}
