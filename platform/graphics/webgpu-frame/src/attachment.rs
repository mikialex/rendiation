use crate::*;

const BASIC_TEXTURE_USAGE_FOR_TEXTURE_POOL: TextureUsages = TextureUsages::TEXTURE_BINDING
  .union(TextureUsages::COPY_DST)
  .union(TextureUsages::COPY_SRC)
  .union(TextureUsages::RENDER_ATTACHMENT);

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct PooledTextureKey {
  pub size: Size,
  pub format: raw_gpu::TextureFormat,
  pub sample_count: u32,
  pub require_mipmaps: bool,
  pub usage: TextureUsages,
}

impl PooledTextureKey {
  pub fn create_directly(self, gpu: &GPU) -> RenderViewContent {
    let mip_level_count = if self.require_mipmaps {
      MipLevelCount::BySize.get_level_count_wgpu(self.size)
    } else {
      1
    };
    let texture = GPUTexture::create(
      raw_gpu::TextureDescriptor {
        label: None,
        size: map_size_gpu(self.size),
        dimension: raw_gpu::TextureDimension::D2,
        format: self.format,
        view_formats: &[],
        usage: self.usage,
        mip_level_count,
        sample_count: self.sample_count,
      },
      &gpu.device,
    );
    let render_view = texture.create_view(raw_gpu::TextureViewDescriptor {
      base_mip_level: 0,
      mip_level_count: Some(1),
      ..Default::default()
    });

    RenderViewContent::from_texture_view(render_view)
  }
}

pub trait RenderTargetViewAttachmentKeyExt {
  /// create a key that describes a texture with the same properties as self
  fn create_attachment_key(&self) -> PooledTextureKey;
}

impl RenderTargetViewAttachmentKeyExt for RenderTargetView {
  fn create_attachment_key(&self) -> PooledTextureKey {
    PooledTextureKey {
      size: self.size(),
      format: self.format(),
      sample_count: self.sample_count(),
      require_mipmaps: self.has_mipmaps(),
      usage: self.usage(),
    }
  }
}

pub type AttachmentPool = ReuseKVPool<PooledTextureKey, RenderViewContent>;

pub fn init_attachment_pool(gpu: &GPU) -> AttachmentPool {
  let gpu = gpu.clone();
  ReuseKVPool::new(move |k: &PooledTextureKey| k.create_directly(&gpu))
}

/// A pooled texture, returned to the pool when dropped
pub struct Attachment(ReuseableItem<PooledTextureKey, RenderViewContent>);

impl RenderTargetViewLike for Attachment {
  fn content(&self) -> &RenderViewContent {
    self.0.item()
  }
}

impl From<Attachment> for RenderTargetView {
  fn from(view: Attachment) -> Self {
    Self::Custom(Arc::new(view))
  }
}

pub trait AttachmentKeyFrameExt {
  fn request(self, ctx: &FrameCtx) -> RenderTargetView;
}

impl AttachmentKeyFrameExt for PooledTextureKey {
  fn request(self, ctx: &FrameCtx) -> RenderTargetView {
    Attachment(ctx.pool.request(&self)).into()
  }
}

pub fn attachment() -> AttachmentDescriptor {
  AttachmentDescriptor {
    format: TextureFormat::Rgba8UnormSrgb,
    sample_count: 1,
    sizer: default_sizer(),
    require_mipmaps: false,
    usage: BASIC_TEXTURE_USAGE_FOR_TEXTURE_POOL,
  }
}

pub fn depth_attachment() -> AttachmentDescriptor {
  AttachmentDescriptor {
    format: TextureFormat::Depth32Float,
    sample_count: 1,
    sizer: default_sizer(),
    require_mipmaps: false,
    usage: BASIC_TEXTURE_USAGE_FOR_TEXTURE_POOL,
  }
}

#[derive(Clone)]
pub struct AttachmentDescriptor {
  pub format: TextureFormat,
  pub sample_count: u32,
  pub sizer: Arc<dyn Fn(Size) -> Size>,
  pub require_mipmaps: bool,
  pub usage: TextureUsages,
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
  pub fn format(mut self, format: TextureFormat) -> Self {
    self.format = format;
    self
  }

  /// enable the mipmap, mipmap is disabled by default.
  #[must_use]
  pub fn require_mipmaps(mut self) -> Self {
    self.require_mipmaps = true;
    self
  }

  #[must_use]
  pub fn extra_usage(mut self, flag: TextureUsages) -> Self {
    self.usage |= flag;
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
      usage: self.usage,
    }
    .request(ctx)
  }
}
