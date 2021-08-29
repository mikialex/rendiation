use rendiation_webgpu::*;

use super::{MaterialBindableItemPair, MaterialBindableResourceUpdate, Scene, Texture2DHandle};

impl MaterialBindableResourceUpdate for Box<dyn WebGPUTexture2dSource> {
  type GPU = WebGPUTexture2d;
  fn update(&self, gpu: &mut Option<Self::GPU>, device: &wgpu::Device, queue: &wgpu::Queue) {
    gpu.get_or_insert_with(|| WebGPUTexture2d::create(&device, queue, self.as_ref()));
  }
}

pub type SceneTexture2D = MaterialBindableItemPair<Box<dyn WebGPUTexture2dSource>, WebGPUTexture2d>;

impl Scene {
  pub fn add_texture2d(&mut self, texture: impl WebGPUTexture2dSource) -> Texture2DHandle {
    self
      .texture_2ds
      .insert(MaterialBindableItemPair::new(Box::new(texture)))
  }
}
