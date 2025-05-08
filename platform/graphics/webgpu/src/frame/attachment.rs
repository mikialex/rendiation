use crate::*;

pub type AttachmentPool = ReuseKVPool<PooledTextureKey, GPUTextureView>;
pub type Attachment = ReuseableItem<PooledTextureKey, GPUTextureView>;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct PooledTextureKey {
  pub size: Size,
  pub format: gpu::TextureFormat,
  pub sample_count: u32,
  pub require_mipmaps: bool,
  pub usage: TextureUsages,
}

const BASIC_TEXTURE_USAGE_FOR_TEXTURE_POOL: TextureUsages = gpu::TextureUsages::TEXTURE_BINDING
  .union(gpu::TextureUsages::COPY_DST)
  .union(gpu::TextureUsages::COPY_SRC)
  .union(gpu::TextureUsages::RENDER_ATTACHMENT);

impl PooledTextureKey {
  pub fn request(self, ctx: &FrameCtx) -> RenderTargetView {
    ctx.pool.request(&self).into()
  }
  pub fn create_directly(self, gpu: &GPU) -> GPUTextureView {
    GPUTexture::create(
      gpu::TextureDescriptor {
        label: None,
        size: map_size_gpu(self.size),
        dimension: gpu::TextureDimension::D2,
        format: self.format,
        view_formats: &[],
        usage: self.usage,
        mip_level_count: 1,
        sample_count: self.sample_count,
      },
      &gpu.device,
    )
    .create_default_view()
  }
}

pub fn attachment() -> AttachmentDescriptor {
  AttachmentDescriptor {
    format: gpu::TextureFormat::Rgba8UnormSrgb,
    sample_count: 1,
    sizer: default_sizer(),
    require_mipmaps: false,
    extra_usage: BASIC_TEXTURE_USAGE_FOR_TEXTURE_POOL,
  }
}

pub fn depth_attachment() -> AttachmentDescriptor {
  AttachmentDescriptor {
    format: gpu::TextureFormat::Depth32Float,
    sample_count: 1,
    sizer: default_sizer(),
    require_mipmaps: false,
    extra_usage: BASIC_TEXTURE_USAGE_FOR_TEXTURE_POOL,
  }
}

pub fn depth_stencil_attachment() -> AttachmentDescriptor {
  AttachmentDescriptor {
    format: gpu::TextureFormat::Depth24PlusStencil8,
    sample_count: 1,
    sizer: default_sizer(),
    require_mipmaps: false,
    extra_usage: BASIC_TEXTURE_USAGE_FOR_TEXTURE_POOL,
  }
}

#[derive(Clone)]
pub struct AttachmentDescriptor {
  pub format: gpu::TextureFormat,
  pub sample_count: u32,
  pub sizer: Arc<dyn Fn(Size) -> Size>,
  pub require_mipmaps: bool,
  pub extra_usage: TextureUsages,
}

pub fn default_sizer() -> Arc<dyn Fn(Size) -> Size> {
  Arc::new(|size| size)
}

pub fn ratio_sizer(ratio: f32) -> impl Fn(Size) -> Size + 'static {
  move |size| {
    let (width, height) = size.into_usize();
    let width = width as f32 * ratio;
    let height = height as f32 * ratio;
    Size::from_usize_pair_min_one((width as usize, height as usize))
  }
}

impl AttachmentDescriptor {
  pub fn use_hdr_if_enabled(mut self, enable_hdr: bool) -> Self {
    if enable_hdr {
      self.format = TextureFormat::Rgba16Float
    }
    self
  }

  #[must_use]
  pub fn format(mut self, format: gpu::TextureFormat) -> Self {
    self.format = format;
    self
  }

  #[must_use]
  pub fn require_mipmaps(mut self) -> Self {
    self.require_mipmaps = true;
    self
  }

  #[must_use]
  pub fn extra_usage(mut self, flag: TextureUsages) -> Self {
    self.extra_usage |= flag;
    self
  }

  #[must_use]
  pub fn sizer(mut self, sizer: impl Fn(Size) -> Size + 'static) -> Self {
    self.sizer = Arc::new(sizer);
    self
  }

  #[must_use]
  pub fn sample_count(mut self, sample_count: u32) -> Self {
    self.sample_count = sample_count;
    self
  }
}

impl AttachmentDescriptor {
  pub fn request(self, ctx: &FrameCtx) -> RenderTargetView {
    let size = ctx.frame_size;
    let size = (self.sizer)(size);

    PooledTextureKey {
      size,
      format: self.format,
      sample_count: self.sample_count,
      require_mipmaps: self.require_mipmaps,
      usage: self.extra_usage,
    }
    .request(ctx)
  }
}

pub fn init_attachment_pool(gpu: &GPU) -> AttachmentPool {
  let gpu = gpu.clone();
  ReuseKVPool::new(move |k: &PooledTextureKey| k.create_directly(&gpu))
}
