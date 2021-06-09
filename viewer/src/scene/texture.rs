use rendiation_texture::Size;

use super::{BindableResource, Scene, Texture2DHandle};

pub struct SceneTexture2D {
  data: Box<dyn SceneTexture2dSource>,
  gpu: Option<SceneTexture2dGpu>,
}

impl SceneTexture2D {
  pub fn get_gpu(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> &SceneTexture2dGpu {
    self.gpu.get_or_insert_with(|| {
      let texture_extent = self.data.gpu_size();
      let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: texture_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: self.data.format(),
        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
      });
      let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
      queue.write_texture(
        wgpu::ImageCopyTexture {
          texture: &texture,
          mip_level: 0,
          origin: wgpu::Origin3d::ZERO,
        },
        self.data.as_bytes(),
        wgpu::ImageDataLayout {
          offset: 0,
          bytes_per_row: Some(self.data.bytes_per_row()),
          rows_per_image: None,
        },
        texture_extent,
      );
      SceneTexture2dGpu {
        texture,
        texture_view,
      }
    })
  }
}

pub trait SceneTexture2dSource: 'static {
  fn format(&self) -> wgpu::TextureFormat;
  fn as_bytes(&self) -> &[u8];
  fn size(&self) -> Size;
  fn byte_per_pixel(&self) -> usize;
  fn bytes_per_row(&self) -> std::num::NonZeroU32 {
    std::num::NonZeroU32::new(self.size().width as u32 * self.byte_per_pixel() as u32).unwrap()
  }
  fn gpu_size(&self) -> wgpu::Extent3d {
    let size = self.size();
    wgpu::Extent3d {
      width: size.width as u32,
      height: size.height as u32,
      depth_or_array_layers: 1,
    }
  }
}

impl SceneTexture2dSource for image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
  fn format(&self) -> wgpu::TextureFormat {
    wgpu::TextureFormat::Rgba8Unorm
  }

  fn byte_per_pixel(&self) -> usize {
    return 4;
  }

  fn as_bytes(&self) -> &[u8] {
    self.as_raw()
  }

  fn size(&self) -> Size {
    Size {
      width: self.width() as usize,
      height: self.height() as usize,
    }
  }
}

impl Scene {
  pub fn add_texture2d(&mut self, texture: impl SceneTexture2dSource) -> Texture2DHandle {
    self.texture_2ds.insert(SceneTexture2D {
      data: Box::new(texture),
      gpu: None,
    })
  }
}
