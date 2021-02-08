use crate::renderer::texture::upload;
use crate::renderer::texture::WGPUTexture;
use crate::{renderer::texture_dimension::*, renderer::texture_format::*, WGPURenderer};

pub struct WGPUTextureCube {
  texture: WGPUTexture,
}

#[allow(clippy::too_many_arguments)]
impl WGPUTextureCube {
  pub fn new(
    renderer: &mut WGPURenderer,
    size: (usize, usize),
    px: &[u8],
    nx: &[u8],
    py: &[u8],
    ny: &[u8],
    pz: &[u8],
    nz: &[u8],
  ) -> Self {
    let size: TextureSize2D = size.into();
    let descriptor = wgpu::TextureDescriptor {
      label: None,
      size: size.to_wgpu(),
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureSize2D::WGPU_CONST,
      format: wgpu::TextureFormat::Rgba8UnormSrgb,
      usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
    };
    let gpu_texture = renderer.device.create_texture(&descriptor);
    let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let texture = WGPUTexture {
      gpu_texture,
      descriptor,
      view,
      size,
      format: TextureFormat::Rgba8UnormSrgb,
    };
    let tex = Self { texture };
    tex.upload_all(renderer, px, nx, py, ny, pz, nz);
    tex
  }

  pub fn upload_all(
    &self,
    renderer: &mut WGPURenderer,
    px: &[u8],
    nx: &[u8],
    py: &[u8],
    ny: &[u8],
    pz: &[u8],
    nz: &[u8],
  ) {
    upload(renderer, &self.texture, px, 0);
    upload(renderer, &self.texture, nx, 1);
    upload(renderer, &self.texture, py, 2);
    upload(renderer, &self.texture, ny, 3);
    upload(renderer, &self.texture, pz, 4);
    upload(renderer, &self.texture, nz, 5);
  }
}
