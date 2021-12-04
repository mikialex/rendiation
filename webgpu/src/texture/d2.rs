use rendiation_texture_types::{Size, TextureOrigin};
use wgpu::util::DeviceExt;

use crate::{
  BindableResource, BindableResourceWgslCodeGen, WebGPUTexture, WebGPUTextureCubeDescriptor,
};

pub trait WebGPUTexture2dSource {
  fn format(&self) -> wgpu::TextureFormat;
  fn as_bytes(&self) -> &[u8];
  fn size(&self) -> Size;
  fn bytes_per_pixel(&self) -> usize;

  fn bytes_per_row_usize(&self) -> usize {
    let width: usize = self.size().width.into();
    width * self.bytes_per_pixel()
  }

  fn bytes_per_row(&self) -> std::num::NonZeroU32 {
    std::num::NonZeroU32::new(
      Into::<usize>::into(self.size().width) as u32 * self.bytes_per_pixel() as u32,
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

  /// It is a webgpu requirement that:
  /// BufferCopyView.layout.bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT == 0
  /// So we calculate padded_width by rounding width
  /// up to the next multiple of wgpu::COPY_BYTES_PER_ROW_ALIGNMENT.
  /// Return width with padding
  fn create_upload_buffer(&self, device: &wgpu::Device) -> (wgpu::Buffer, Size) {
    let width: usize = self.size().width.into();

    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padding_size = (align - width % align) % align;

    let buffer = if padding_size == 0 {
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: self.as_bytes(),
        usage: wgpu::BufferUsages::COPY_SRC,
      })
    } else {
      // will this be optimized well or we should just use copy_from_slice?
      let padded_data: Vec<_> = self
        .as_bytes()
        .chunks_exact(self.bytes_per_row_usize())
        .flat_map(|row| row.iter().copied().chain((0..padding_size).map(|_| 0)))
        .collect();

      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: padded_data.as_slice(),
        usage: wgpu::BufferUsages::COPY_SRC,
      })
    };

    let size = Size::from_usize_pair_min_one((width + padding_size, self.size().height.into()));

    (buffer, size)
  }

  fn create_tex2d_desc(&self, level_count: MipLevelCount) -> WebGPUTexture2dDescriptor {
    // todo validation;
    WebGPUTexture2dDescriptor {
      desc: wgpu::TextureDescriptor {
        label: None,
        size: self.gpu_size(),
        mip_level_count: level_count.get_level_count_wgpu(self.size()),
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: self.format(),
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
      },
    }
  }

  fn create_cube_desc(&self, level_count: MipLevelCount) -> WebGPUTextureCubeDescriptor {
    // todo validation;
    WebGPUTextureCubeDescriptor {
      desc: wgpu::TextureDescriptor {
        label: None,
        size: self.gpu_cube_size(),
        mip_level_count: level_count.get_level_count_wgpu(self.size()),
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: self.format(),
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
      },
    }
  }
}

/// The wrapper type that make sure the inner desc
/// is suitable for 2d texture
pub struct WebGPUTexture2dDescriptor {
  desc: wgpu::TextureDescriptor<'static>,
}

impl WebGPUTexture2dDescriptor {
  pub fn from_size(size: Size) -> Self {
    Self {
      desc: wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
          width: Into::<usize>::into(size.width) as u32,
          height: Into::<usize>::into(size.height) as u32,
          depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
      },
    }
  }

  pub fn with_format(mut self, format: wgpu::TextureFormat) -> Self {
    self.desc.format = format;
    self
  }

  pub fn with_level_count(mut self, level_count: MipLevelCount) -> Self {
    self.desc.mip_level_count = level_count.get_level_count_wgpu(Size::from_u32_pair_min_one((
      self.desc.size.width,
      self.desc.size.height,
    )));
    self
  }
}

pub enum MipLevelCount {
  BySize,
  EmptyMipMap,
  Fixed(usize),
}

impl MipLevelCount {
  #[allow(clippy::let_and_return)]
  pub fn get_level_count_wgpu(&self, size: Size) -> u32 {
    let r = match *self {
      MipLevelCount::BySize => size.mip_level_count(),
      MipLevelCount::EmptyMipMap => 1,
      // todo should we do validation?
      MipLevelCount::Fixed(s) => s,
    } as u32;
    r
  }
}

pub struct WebGPUTexture2d {
  pub(crate) texture: WebGPUTexture,
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

impl BindableResourceWgslCodeGen for WebGPUTexture2d {
  fn wgsl_type_name(layout: wgpu::BindingType) -> &'static str {
    "texture_cube<f32>"
  }
}

impl WebGPUTexture2d {
  pub fn create(device: &wgpu::Device, desc: WebGPUTexture2dDescriptor) -> Self {
    let desc = desc.desc;
    let texture = device.create_texture(&desc);
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let texture = WebGPUTexture { texture, desc };

    Self {
      texture,
      texture_view,
    }
  }

  pub fn get_default_view(&self) -> &wgpu::TextureView {
    &self.texture_view
  }

  pub fn upload(
    &self,
    queue: &wgpu::Queue,
    source: &dyn WebGPUTexture2dSource,
    mip_level: usize,
  ) -> &Self {
    self.upload_with_origin(queue, source, mip_level, TextureOrigin::zero())
  }

  pub fn upload_into(
    self,
    queue: &wgpu::Queue,
    source: &dyn WebGPUTexture2dSource,
    mip_level: usize,
  ) -> Self {
    self.upload_with_origin(queue, source, mip_level, TextureOrigin::zero());
    self
  }

  pub fn upload_with_origin(
    &self,
    queue: &wgpu::Queue,
    source: &dyn WebGPUTexture2dSource,
    mip_level: usize,
    origin: TextureOrigin,
  ) -> &Self {
    queue.write_texture(
      wgpu::ImageCopyTexture {
        texture: &self.texture,
        mip_level: mip_level as u32,
        origin: wgpu::Origin3d {
          x: origin.x as u32,
          y: origin.y as u32,
          z: 0,
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

  pub fn upload_with_origin_into(
    self,
    queue: &wgpu::Queue,
    source: &dyn WebGPUTexture2dSource,
    mip_level: usize,
    origin: TextureOrigin,
  ) -> Self {
    self.upload_with_origin(queue, source, mip_level, origin);
    self
  }
}
