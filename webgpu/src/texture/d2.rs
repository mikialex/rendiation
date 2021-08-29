use rendiation_texture::Size;
use std::num::NonZeroUsize;

use crate::{BindableResource, WebGPUTexture};

pub trait WebGPUTexture2dSource: 'static {
  fn format(&self) -> wgpu::TextureFormat;
  fn as_bytes(&self) -> &[u8];
  fn size(&self) -> Size;
  fn byte_per_pixel(&self) -> usize;
  fn bytes_per_row(&self) -> std::num::NonZeroU32 {
    std::num::NonZeroU32::new(
      Into::<usize>::into(self.size().width) as u32 * self.byte_per_pixel() as u32,
    )
    .unwrap()
  }
  fn gpu_size(&self) -> wgpu::Extent3d {
    let size = self.size();
    wgpu::Extent3d {
      width: Into::<usize>::into(size.width) as u32,
      height: Into::<usize>::into(size.height) as u32,
      depth_or_array_layers: 1,
    }
  }
  fn gpu_cube_size(&self) -> wgpu::Extent3d {
    let size = self.size();
    wgpu::Extent3d {
      width: Into::<usize>::into(size.width) as u32,
      height: Into::<usize>::into(size.height) as u32,
      depth_or_array_layers: 6,
    }
  }
}

impl WebGPUTexture2dSource for image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
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
      width: NonZeroUsize::new(self.width() as usize).unwrap(),
      height: NonZeroUsize::new(self.height() as usize).unwrap(),
    }
  }
}

pub struct WebGPUTexture2d {
  texture: WebGPUTexture,
  texture_view: wgpu::TextureView,
}

impl BindableResource for WebGPUTexture2d {
  fn as_bindable(&self) -> wgpu::BindingResource {
    wgpu::BindingResource::TextureView(&self.texture_view)
  }

  fn bind_layout() -> wgpu::BindingType {
    wgpu::BindingType::Texture {
      multisampled: false,
      sample_type: wgpu::TextureSampleType::Float { filterable: true },
      view_dimension: wgpu::TextureViewDimension::D2,
    }
  }
}

impl WebGPUTexture2d {
  pub fn create(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    base_source: &dyn WebGPUTexture2dSource,
  ) -> Self {
    let texture_extent = base_source.gpu_size();
    let desc = wgpu::TextureDescriptor {
      label: None,
      size: texture_extent,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: base_source.format(),
      usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
    };
    let texture = device.create_texture(&desc);
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let texture = WebGPUTexture { texture, desc };

    let tex = WebGPUTexture2d {
      texture,
      texture_view,
    };

    tex.upload_base(queue, base_source);

    tex
  }

  pub fn upload_base(&self, queue: &wgpu::Queue, source: &dyn WebGPUTexture2dSource) -> &Self {
    queue.write_texture(
      wgpu::ImageCopyTexture {
        texture: &self.texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
      },
      source.as_bytes(),
      wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(source.bytes_per_row()),
        rows_per_image: None,
      },
      self.texture.desc.size,
    );
    self
  }
}
