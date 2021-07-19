use std::num::NonZeroUsize;

use rendiation_texture::Size;

use super::BindableResource;

pub trait SceneTexture2dSource: 'static {
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
      width: NonZeroUsize::new(self.width() as usize).unwrap(),
      height: NonZeroUsize::new(self.height() as usize).unwrap(),
    }
  }
}

pub struct SceneTexture2dGpu {
  texture: wgpu::Texture,
  texture_view: wgpu::TextureView,
}

impl BindableResource for SceneTexture2dGpu {
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

impl SceneTexture2dGpu {
  pub fn create(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &dyn SceneTexture2dSource,
  ) -> Self {
    let texture_extent = source.gpu_size();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
      label: None,
      size: texture_extent,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: source.format(),
      usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    queue.write_texture(
      wgpu::ImageCopyTexture {
        texture: &texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
      },
      source.as_bytes(),
      wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(source.bytes_per_row()),
        rows_per_image: None,
      },
      texture_extent,
    );
    SceneTexture2dGpu {
      texture,
      texture_view,
    }
  }
}

pub struct SceneTextureCubeGPU {
  texture: wgpu::Texture,
  texture_view: wgpu::TextureView,
}

impl BindableResource for SceneTextureCubeGPU {
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

impl SceneTextureCubeGPU {
  pub fn create(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: [&dyn SceneTexture2dSource; 6],
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

    // SceneTextureCubeGPU {
    //   texture,
    //   texture_view,
    // }
  }
}
