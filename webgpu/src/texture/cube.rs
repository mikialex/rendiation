use rendiation_texture::CubeTextureFace;

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
  pub fn create(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: [&dyn WebGPUTexture2dSource; 6],
  ) -> Self {
    // check source is valid
    let size = source[0].size();
    let format = source[0].format();
    assert_eq!(size.width, size.height);
    source.iter().for_each(|s| {
      assert_eq!(s.size(), size);
      assert_eq!(s.format(), format);
    });

    let texture_extent = source[0].gpu_cube_size();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
      label: None,
      size: texture_extent,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format,
      usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
      label: None,
      dimension: Some(wgpu::TextureViewDimension::Cube),
      ..wgpu::TextureViewDescriptor::default()
    });

    todo!()

    // source.iter().enumerate().for_each(|(i, source)| {
    //   queue.write_texture(
    //     wgpu::ImageCopyTexture {
    //       texture: &texture,
    //       mip_level: 0,
    //       origin: wgpu::Origin3d {
    //         x: 0,
    //         y: 0,
    //         z: i as u32,
    //       },
    //     },
    //     source.as_bytes(),
    //     wgpu::ImageDataLayout {
    //       offset: 0,
    //       bytes_per_row: Some(source.bytes_per_row()),
    //       rows_per_image: None,
    //     },
    //     texture_extent,
    //   );
    // });

    // WebGPUTextureCube {
    //   texture,
    //   texture_view,
    // }
  }

  pub fn upload_with_origin(
    self,
    queue: &wgpu::Queue,
    face: CubeTextureFace,
    mip_level: usize,
    origin: wgpu::Origin3d,
  ) -> Self {
    self
  }
}
