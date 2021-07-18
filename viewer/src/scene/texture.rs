use rendiation_webgpu::*;

use super::{MaterialBindableItemPair, MaterialBindableResourceUpdate, Scene, Texture2DHandle};

impl MaterialBindableResourceUpdate for Box<dyn SceneTexture2dSource> {
  type GPU = SceneTexture2dGpu;
  fn update(&self, gpu: &mut Option<Self::GPU>, device: &wgpu::Device, queue: &wgpu::Queue) {
    gpu.get_or_insert_with(|| SceneTexture2dGpu::create(&device, queue, self.as_ref()));
  }
}

pub type SceneTexture2D =
  MaterialBindableItemPair<Box<dyn SceneTexture2dSource>, SceneTexture2dGpu>;

impl Scene {
  pub fn add_texture2d(&mut self, texture: impl SceneTexture2dSource) -> Texture2DHandle {
    self
      .texture_2ds
      .insert(MaterialBindableItemPair::new(Box::new(texture)))
  }
}
