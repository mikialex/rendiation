use rendiation_texture_types::CubeTextureFace;

use crate::{BindableResource, WebGPUTexture, WebGPUTexture2dSource};

/// The wrapper type that make sure the inner desc
/// is suitable for cube texture
pub struct WebGPUTextureCubeDescriptor {
  pub(crate) desc: wgpu::TextureDescriptor<'static>,
}

pub struct WebGPUTextureCube {
  texture: WebGPUTexture,
  texture_view: wgpu::TextureView,
}

impl BindableResource for WebGPUTextureCube {
  fn as_bindable(&self) -> wgpu::BindingResource {
    wgpu::BindingResource::TextureView(&self.texture_view)
  }
  fn bind_layout() -> wgpu::BindingType {
    wgpu::BindingType::Texture {
      multisampled: false,
      sample_type: wgpu::TextureSampleType::Float { filterable: true },
      view_dimension: wgpu::TextureViewDimension::Cube,
    }
  }
}

impl WebGPUTextureCube {
  pub fn create(device: &wgpu::Device, desc: WebGPUTextureCubeDescriptor) -> Self {
    let desc = desc.desc;

    let texture = device.create_texture(&desc);
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
      dimension: Some(wgpu::TextureViewDimension::Cube),
      ..wgpu::TextureViewDescriptor::default()
    });

    let texture = WebGPUTexture { texture, desc };

    let tex = Self {
      texture,
      texture_view,
    };

    tex
  }

  pub fn upload(
    self,
    queue: &wgpu::Queue,
    source: &dyn WebGPUTexture2dSource,
    face: CubeTextureFace,
    mip_level: usize,
  ) -> Self {
    self.upload_with_origin(queue, source, face, mip_level, (0, 0))
  }

  pub fn upload_with_origin(
    self,
    queue: &wgpu::Queue,
    source: &dyn WebGPUTexture2dSource,
    face: CubeTextureFace,
    mip_level: usize,
    origin: (usize, usize),
  ) -> Self {
    // validation
    queue.write_texture(
      wgpu::ImageCopyTexture {
        texture: &self.texture,
        mip_level: mip_level as u32,
        origin: wgpu::Origin3d {
          x: origin.0 as u32,
          y: origin.1 as u32,
          z: face as u32,
        },
        aspect: wgpu::TextureAspect::All,
      },
      source.as_bytes(),
      wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(source.bytes_per_row()),
        rows_per_image: None,
      },
      source.gpu_size(),
    );
    self
  }
}
