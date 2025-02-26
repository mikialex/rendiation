use crate::*;

pub type AttachmentPool = ReuseKVPool<PooledTextureKey, GPU2DTextureView>;
pub type Attachment = ReuseableItem<PooledTextureKey, GPU2DTextureView>;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct PooledTextureKey {
  size: Size,
  format: gpu::TextureFormat,
  sample_count: u32,
}

pub fn attachment() -> AttachmentDescriptor {
  AttachmentDescriptor {
    format: gpu::TextureFormat::Rgba8UnormSrgb,
    sample_count: 1,
    sizer: default_sizer(),
  }
}

pub fn depth_attachment() -> AttachmentDescriptor {
  AttachmentDescriptor {
    format: gpu::TextureFormat::Depth32Float,
    sample_count: 1,
    sizer: default_sizer(),
  }
}

pub fn depth_stencil_attachment() -> AttachmentDescriptor {
  AttachmentDescriptor {
    format: gpu::TextureFormat::Depth24PlusStencil8,
    sample_count: 1,
    sizer: default_sizer(),
  }
}

#[derive(Clone)]
pub struct AttachmentDescriptor {
  pub(super) format: gpu::TextureFormat,
  pub(super) sample_count: u32,
  pub(super) sizer: Arc<dyn Fn(Size) -> Size>,
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
  #[must_use]
  pub fn format(mut self, format: gpu::TextureFormat) -> Self {
    self.format = format;
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
  pub fn request(self, ctx: &FrameCtx) -> Attachment {
    let size = ctx.frame_size;
    let size = (self.sizer)(size);

    let key = PooledTextureKey {
      size,
      format: self.format,
      sample_count: self.sample_count,
    };

    ctx.pool.request(&key)
  }
}

pub fn init_attachment_pool(gpu: &GPU) -> AttachmentPool {
  let gpu = gpu.clone();
  ReuseKVPool::new(move |k: &PooledTextureKey| {
    let tex: GPU2DTexture = GPUTexture::create(
      gpu::TextureDescriptor {
        label: None,
        size: map_size_gpu(k.size),
        dimension: gpu::TextureDimension::D2,
        format: k.format,
        view_formats: &[],
        usage: gpu::TextureUsages::TEXTURE_BINDING
          | gpu::TextureUsages::COPY_DST
          | gpu::TextureUsages::COPY_SRC
          | gpu::TextureUsages::RENDER_ATTACHMENT,
        mip_level_count: 1,
        sample_count: k.sample_count,
      },
      &gpu.device,
    )
    .try_into()
    .unwrap();
    tex.create_default_view().try_into().unwrap()
  })
}
