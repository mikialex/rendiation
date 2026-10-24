use crate::*;

// for shortcut
pub fn create_empty_2d_texture_view(
  gpu: &GPU,
  size: Size,
  usage: gpu::TextureUsages,
  format: gpu::TextureFormat,
) -> GPU2DTextureView {
  let tex = GPUTexture::create(
    TextureDescriptor {
      label: None,
      size: size.into_gpu_size(),
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureDimension::D2,
      format,
      view_formats: &[],
      usage,
    },
    &gpu.device,
  );
  let tex = GPU2DTexture::try_from(tex).unwrap().create_default_view();
  GPU2DTextureView::try_from(tex).unwrap()
}

pub fn map_size_gpu(size: Size) -> gpu::Extent3d {
  gpu::Extent3d {
    width: Into::<usize>::into(size.width) as u32,
    height: Into::<usize>::into(size.height) as u32,
    depth_or_array_layers: 1,
  }
}

/// The texture copy related operations require the format to be copyable, see
/// [gpu::TextureFormat::block_copy_size]
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("texture format {format:?} with aspect {aspect:?} is not copyable")]
pub struct TextureFormatNotCopyable {
  pub format: gpu::TextureFormat,
  pub aspect: gpu::TextureAspect,
}

pub(crate) fn expect_copyable<T>(result: Result<T, TextureFormatNotCopyable>) -> T {
  result.unwrap_or_else(|err| panic!("{err}"))
}

/// The upload methods without try prefix panic if the source format is not copyable.
impl GPU2DTexture {
  pub fn upload(
    &self,
    queue: &GPUQueue,
    source: &dyn WebGPU2DTextureSource,
    mip_level: usize,
  ) -> &Self {
    expect_copyable(self.try_upload(queue, source, mip_level))
  }

  pub fn try_upload(
    &self,
    queue: &GPUQueue,
    source: &dyn WebGPU2DTextureSource,
    mip_level: usize,
  ) -> Result<&Self, TextureFormatNotCopyable> {
    self.try_upload_with_origin(queue, source, mip_level, TextureOrigin::zero())
  }

  #[must_use]
  pub fn upload_into(
    self,
    queue: &GPUQueue,
    source: &dyn WebGPU2DTextureSource,
    mip_level: usize,
  ) -> Self {
    self.upload(queue, source, mip_level);
    self
  }

  pub fn upload_with_origin(
    &self,
    queue: &GPUQueue,
    source: &dyn WebGPU2DTextureSource,
    mip_level: usize,
    origin: TextureOrigin,
  ) -> &Self {
    expect_copyable(self.try_upload_with_origin(queue, source, mip_level, origin))
  }

  pub fn try_upload_with_origin(
    &self,
    queue: &GPUQueue,
    source: &dyn WebGPU2DTextureSource,
    mip_level: usize,
    origin: TextureOrigin,
  ) -> Result<&Self, TextureFormatNotCopyable> {
    let bytes_per_row = source.bytes_per_row()?;
    queue.write_texture(
      gpu::TexelCopyTextureInfo {
        texture: self.texture.gpu_resource().gpu(),
        mip_level: mip_level as u32,
        origin: gpu::Origin3d {
          x: origin.x as u32,
          y: origin.y as u32,
          z: 0,
        },
        aspect: gpu::TextureAspect::All,
      },
      source.as_bytes(),
      gpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(bytes_per_row),
        rows_per_image: None,
      },
      source.gpu_size(),
    );
    Ok(self)
  }

  #[must_use]
  pub fn upload_with_origin_into(
    self,
    queue: &GPUQueue,
    source: &dyn WebGPU2DTextureSource,
    mip_level: usize,
    origin: TextureOrigin,
  ) -> Self {
    self.upload_with_origin(queue, source, mip_level, origin);
    self
  }
}

pub trait WebGPU2DTextureSource: Send + Sync {
  fn format(&self) -> gpu::TextureFormat;
  fn as_bytes(&self) -> &[u8];
  fn size(&self) -> Size;

  /// The bytes of one texel block, for uncompressed format it's the bytes per pixel.
  fn bytes_per_block(&self) -> Result<usize, TextureFormatNotCopyable> {
    let format = self.format();
    format
      .block_copy_size(None)
      .map(|size| size as usize)
      .ok_or(TextureFormatNotCopyable {
        format,
        aspect: gpu::TextureAspect::All,
      })
  }

  /// The texel block row count, for uncompressed format it's the height.
  fn block_rows(&self) -> usize {
    let (_, block_height) = self.format().block_dimensions();
    usize::from(self.size().height).div_ceil(block_height as usize)
  }

  fn bytes_per_row_usize(&self) -> Result<usize, TextureFormatNotCopyable> {
    let (block_width, _) = self.format().block_dimensions();
    let block_count = usize::from(self.size().width).div_ceil(block_width as usize);
    Ok(block_count * self.bytes_per_block()?)
  }

  fn bytes_per_row(&self) -> Result<u32, TextureFormatNotCopyable> {
    self.bytes_per_row_usize().map(|v| v as u32)
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
  /// Return the buffer and the padded bytes per row
  fn create_upload_buffer(
    &self,
    device: &GPUDevice,
  ) -> Result<(gpu::Buffer, usize), TextureFormatNotCopyable> {
    let width = self.bytes_per_row_usize()?;

    let align = gpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padding_size = (align - width % align) % align;

    let buffer = if padding_size == 0 {
      device.create_buffer_init(&gpu::util::BufferInitDescriptor {
        label: None,
        contents: self.as_bytes(),
        usage: gpu::BufferUsages::COPY_SRC,
      })
    } else {
      // will this be optimized well, or we should just use copy_from_slice?
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

    Ok((buffer, width + padding_size))
  }

  fn create_tex2d_desc(
    &self,
    level_count: MipLevelCount,
    flags: DownlevelFlags,
  ) -> gpu::TextureDescriptor<'static> {
    gpu::TextureDescriptor {
      label: None,
      size: self.gpu_size(),
      mip_level_count: level_count.get_level_count_wgpu(self.size()),
      sample_count: 1,
      dimension: gpu::TextureDimension::D2,
      format: self.format(),
      view_formats: get_view_format(self.format(), flags),
      usage: gpu::TextureUsages::TEXTURE_BINDING
        | gpu::TextureUsages::RENDER_ATTACHMENT
        // | gpu::TextureUsages::STORAGE_BINDING // used to generate mipmap in compute shader
        | gpu::TextureUsages::COPY_DST
        | gpu::TextureUsages::COPY_SRC,
    }
  }

  fn create_cube_desc(
    &self,
    level_count: MipLevelCount,
    flags: DownlevelFlags,
  ) -> gpu::TextureDescriptor<'static> {
    gpu::TextureDescriptor {
      label: None,
      size: self.gpu_cube_size(),
      view_formats: get_view_format(self.format(), flags),
      mip_level_count: level_count.get_level_count_wgpu(self.size()),
      sample_count: 1,
      dimension: gpu::TextureDimension::D2,
      format: self.format(),
      usage: gpu::TextureUsages::TEXTURE_BINDING
        | gpu::TextureUsages::RENDER_ATTACHMENT
        | gpu::TextureUsages::COPY_DST
        | gpu::TextureUsages::COPY_SRC,
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

fn get_view_format(format: TextureFormat, flags: DownlevelFlags) -> &'static [TextureFormat] {
  if !flags.contains(DownlevelFlags::VIEW_FORMATS) {
    return &[];
  }

  match format {
    TextureFormat::Rgba8UnormSrgb => &[TextureFormat::Rgba8UnormSrgb, TextureFormat::Rgba8Unorm],
    TextureFormat::Rgba8Unorm => &[TextureFormat::Rgba8UnormSrgb, TextureFormat::Rgba8Unorm],
    TextureFormat::Bgra8UnormSrgb => &[TextureFormat::Bgra8UnormSrgb, TextureFormat::Bgra8Unorm],
    TextureFormat::Bgra8Unorm => &[TextureFormat::Bgra8UnormSrgb, TextureFormat::Bgra8Unorm],
    _ => &[],
  }
}

#[test]
fn test_texture_source_bytes_per_row() {
  struct Source(gpu::TextureFormat);
  impl WebGPU2DTextureSource for Source {
    fn format(&self) -> gpu::TextureFormat {
      self.0
    }
    fn as_bytes(&self) -> &[u8] {
      &[]
    }
    fn size(&self) -> Size {
      Size::from_u32_pair_min_one((10, 6))
    }
  }

  let rgba = Source(gpu::TextureFormat::Rgba8Unorm);
  assert_eq!(rgba.bytes_per_row().unwrap(), 40);
  assert_eq!(rgba.block_rows(), 6);

  // the compressed format is counted by 4x4 blocks
  let bc1 = Source(gpu::TextureFormat::Bc1RgbaUnorm);
  assert_eq!(bc1.bytes_per_row().unwrap(), 3 * 8);
  assert_eq!(bc1.block_rows(), 2);

  assert!(
    Source(gpu::TextureFormat::Depth24Plus)
      .bytes_per_row()
      .is_err()
  );
}
