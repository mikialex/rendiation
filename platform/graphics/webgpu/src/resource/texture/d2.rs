use crate::*;

pub fn map_size_gpu(size: Size) -> gpu::Extent3d {
  gpu::Extent3d {
    width: Into::<usize>::into(size.width) as u32,
    height: Into::<usize>::into(size.height) as u32,
    depth_or_array_layers: 1,
  }
}

impl GPU2DTexture {
  pub fn upload(
    &self,
    queue: &gpu::Queue,
    source: &dyn WebGPU2DTextureSource,
    mip_level: usize,
  ) -> &Self {
    self.upload_with_origin(queue, source, mip_level, TextureOrigin::zero())
  }

  #[must_use]
  pub fn upload_into(
    self,
    queue: &gpu::Queue,
    source: &dyn WebGPU2DTextureSource,
    mip_level: usize,
  ) -> Self {
    self.upload_with_origin(queue, source, mip_level, TextureOrigin::zero());
    self
  }

  pub fn upload_with_origin(
    &self,
    queue: &gpu::Queue,
    source: &dyn WebGPU2DTextureSource,
    mip_level: usize,
    origin: TextureOrigin,
  ) -> &Self {
    queue.write_texture(
      gpu::ImageCopyTexture {
        texture: &self.0.inner.resource,
        mip_level: mip_level as u32,
        origin: gpu::Origin3d {
          x: origin.x as u32,
          y: origin.y as u32,
          z: 0,
        },
        aspect: gpu::TextureAspect::All,
      },
      source.as_bytes(),
      gpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(source.bytes_per_row()),
        rows_per_image: None,
      },
      source.gpu_size(),
    );
    self
  }

  #[must_use]
  pub fn upload_with_origin_into(
    self,
    queue: &gpu::Queue,
    source: &dyn WebGPU2DTextureSource,
    mip_level: usize,
    origin: TextureOrigin,
  ) -> Self {
    self.upload_with_origin(queue, source, mip_level, origin);
    self
  }
}

define_dyn_trait_downcaster_static!(WebGPU2DTextureSource);
pub trait WebGPU2DTextureSource: Send + Sync {
  fn format(&self) -> gpu::TextureFormat;
  fn as_bytes(&self) -> &[u8];
  fn size(&self) -> Size;
  fn bytes_per_pixel(&self) -> usize {
    self.format().block_size(None).unwrap() as usize
  }

  fn bytes_per_row_usize(&self) -> usize {
    let width: usize = self.size().width.into();
    width * self.bytes_per_pixel()
  }

  fn bytes_per_row(&self) -> u32 {
    Into::<usize>::into(self.size().width) as u32 * self.bytes_per_pixel() as u32
  }

  fn gpu_size(&self) -> gpu::Extent3d {
    let size = self.size();
    gpu::Extent3d {
      width: Into::<usize>::into(size.width) as u32,
      height: Into::<usize>::into(size.height) as u32,
      depth_or_array_layers: 1,
    }
  }

  fn gpu_cube_size(&self) -> gpu::Extent3d {
    let size = self.size();
    gpu::Extent3d {
      width: Into::<usize>::into(size.width) as u32,
      height: Into::<usize>::into(size.height) as u32,
      depth_or_array_layers: 6,
    }
  }

  /// It is a webgpu requirement that:
  /// BufferCopyView.layout.bytes_per_row % gpu::COPY_BYTES_PER_ROW_ALIGNMENT == 0
  /// So we calculate padded_width by rounding width
  /// up to the next multiple of gpu::COPY_BYTES_PER_ROW_ALIGNMENT.
  /// Return width with padding
  fn create_upload_buffer(&self, device: &GPUDevice) -> (gpu::Buffer, Size) {
    let width = self.bytes_per_row_usize();

    let align = gpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padding_size = (align - width % align) % align;

    let buffer = if padding_size == 0 {
      device.create_buffer_init(&gpu::util::BufferInitDescriptor {
        label: None,
        contents: self.as_bytes(),
        usage: gpu::BufferUsages::COPY_SRC,
      })
    } else {
      // will this be optimized well or we should just use copy_from_slice?
      let padded_data: Vec<_> = self
        .as_bytes()
        .chunks_exact(width)
        .flat_map(|row| row.iter().copied().chain((0..padding_size).map(|_| 0)))
        .collect();

      device.create_buffer_init(&gpu::util::BufferInitDescriptor {
        label: None,
        contents: padded_data.as_slice(),
        usage: gpu::BufferUsages::COPY_SRC,
      })
    };

    let size = Size::from_usize_pair_min_one((width + padding_size, self.size().height.into()));

    (buffer, size)
  }

  fn create_tex2d_desc(&self, level_count: MipLevelCount) -> gpu::TextureDescriptor<'static> {
    gpu::TextureDescriptor {
      label: None,
      size: self.gpu_size(),
      mip_level_count: level_count.get_level_count_wgpu(self.size()),
      sample_count: 1,
      dimension: gpu::TextureDimension::D2,
      format: self.format(),
      view_formats: &[],
      usage: gpu::TextureUsages::TEXTURE_BINDING
        | gpu::TextureUsages::RENDER_ATTACHMENT
        | gpu::TextureUsages::COPY_DST,
    }
  }

  fn create_cube_desc(&self, level_count: MipLevelCount) -> gpu::TextureDescriptor<'static> {
    gpu::TextureDescriptor {
      label: None,
      size: self.gpu_cube_size(),
      view_formats: &[],
      mip_level_count: level_count.get_level_count_wgpu(self.size()),
      sample_count: 1,
      dimension: gpu::TextureDimension::D2,
      format: self.format(),
      usage: gpu::TextureUsages::TEXTURE_BINDING
        | gpu::TextureUsages::RENDER_ATTACHMENT
        | gpu::TextureUsages::COPY_DST,
    }
  }
}

pub enum MipLevelCount {
  BySize,
  EmptyMipMap,
  Fixed(usize),
}

impl MipLevelCount {
  pub fn get_level_count_wgpu(&self, size: Size) -> u32 {
    match *self {
      MipLevelCount::BySize => size.mip_level_count() as u32,
      MipLevelCount::EmptyMipMap => 1,
      // todo should we do validation?
      MipLevelCount::Fixed(s) => s as u32,
    }
  }
}
