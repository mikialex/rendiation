use rendiation_webgpu::*;

use crate::{TextureCubeHandle, TextureCubeSource};

use super::{MaterialBindableItemPair, MaterialBindableResourceUpdate, Scene, Texture2DHandle};

impl MaterialBindableResourceUpdate for Box<dyn WebGPUTexture2dSource> {
  type GPU = WebGPUTexture2d;
  fn update(&self, gpu: &mut Option<Self::GPU>, device: &wgpu::Device, queue: &wgpu::Queue) {
    gpu.get_or_insert_with(|| {
      let source = self.as_ref();
      let desc = source.create_tex2d_desc(MipLevelCount::EmptyMipMap);

      WebGPUTexture2d::create(device, desc).upload_into(queue, source, 0)
    });
  }
}

pub type SceneTexture2D = MaterialBindableItemPair<Box<dyn WebGPUTexture2dSource>, WebGPUTexture2d>;

impl Scene {
  pub fn add_texture2d(
    &mut self,
    texture: impl WebGPUTexture2dSource + 'static,
  ) -> Texture2DHandle {
    self
      .texture_2ds
      .insert(MaterialBindableItemPair::new(Box::new(texture)))
  }

  pub fn add_texture_cube(&mut self, texture: TextureCubeSource) -> TextureCubeHandle {
    self
      .texture_cubes
      .insert(MaterialBindableItemPair::new(texture))
  }
}
