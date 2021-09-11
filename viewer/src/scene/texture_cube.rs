use rendiation_texture::CubeTextureFace;
use rendiation_webgpu::*;

use crate::{MaterialBindableItemPair, MaterialBindableResourceUpdate};

pub type TextureCubeSource = [Box<dyn WebGPUTexture2dSource>; 6];
pub type SceneTextureCube = MaterialBindableItemPair<TextureCubeSource, WebGPUTextureCube>;

impl MaterialBindableResourceUpdate for TextureCubeSource {
  type GPU = WebGPUTextureCube;
  fn update(&self, gpu: &mut Option<Self::GPU>, device: &wgpu::Device, queue: &wgpu::Queue) {
    gpu.get_or_insert_with(|| {
      let source = self.as_ref();
      let desc = source[0].create_cube_desc(MipLevelCount::EmptyMipMap);

      WebGPUTextureCube::create(device, desc)
        .upload(queue, source[0].as_ref(), CubeTextureFace::PositiveX, 0)
        .upload(queue, source[1].as_ref(), CubeTextureFace::NegativeX, 0)
        .upload(queue, source[2].as_ref(), CubeTextureFace::PositiveY, 0)
        .upload(queue, source[3].as_ref(), CubeTextureFace::NegativeY, 0)
        .upload(queue, source[4].as_ref(), CubeTextureFace::PositiveZ, 0)
        .upload(queue, source[5].as_ref(), CubeTextureFace::NegativeZ, 0)
    });
  }
}
