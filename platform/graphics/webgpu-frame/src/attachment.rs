use crate::*;

const BASIC_TEXTURE_USAGE_FOR_TEXTURE_POOL: TextureUsages = TextureUsages::TEXTURE_BINDING
  .union(TextureUsages::COPY_DST)
  .union(TextureUsages::COPY_SRC)
  .union(TextureUsages::RENDER_ATTACHMENT);

pub trait AttachmentKeyFrameExt {
  fn request(self, ctx: &FrameCtx) -> RenderTargetView;
}

impl AttachmentKeyFrameExt for PooledTextureKey {
  fn request(self, ctx: &FrameCtx) -> RenderTargetView {
    ctx.pool.request(&self).into()
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
