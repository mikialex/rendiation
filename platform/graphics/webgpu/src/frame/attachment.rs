use rendiation_shader_api::*;

use crate::*;

#[derive(Clone, Default)]
pub struct AttachmentPool {
  pub inner: Arc<RwLock<AttachmentPoolImpl>>,
}

impl AttachmentPool {
  pub fn clear(&mut self) {
    self.inner.write().unwrap().clear()
  }
  pub fn set_enable_reusing(&mut self, enable_reusing: bool) {
    self
      .inner
      .write()
      .unwrap()
      .set_enable_reusing(enable_reusing)
  }
}

#[derive(Default)]
pub struct AttachmentPoolImpl {
  attachments: FastHashMap<PooledTextureKey, SingleAttachmentPool>,
  enable_reusing: bool,
}

impl AttachmentPoolImpl {
  pub fn clear(&mut self) {
    self.attachments.clear();
  }
  pub fn set_enable_reusing(&mut self, enable_reusing: bool) {
    self.enable_reusing = enable_reusing;
    self.attachments.clear();
  }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct PooledTextureKey {
  size: Size,
  format: gpu::TextureFormat,
  sample_count: u32,
}

#[derive(Default)]
struct SingleAttachmentPool {
  cached: Vec<GPU2DTexture>,
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

/// Ownership is always transferred, do not support clone.
pub struct Attachment {
  pool: AttachmentPool,
  des: AttachmentDescriptor,
  texture: GPU2DTexture,
  key: PooledTextureKey,
}

impl AsRef<GPU2DTexture> for Attachment {
  fn as_ref(&self) -> &GPU2DTexture {
    &self.texture
  }
}

impl AsRef<Attachment> for Attachment {
  fn as_ref(&self) -> &Attachment {
    self
  }
}

/// When it drops, return the texture to the reusing pool;
impl Drop for Attachment {
  fn drop(&mut self) {
    let mut pool = self.pool.inner.write().unwrap();
    if pool.enable_reusing {
      let pool = pool
        .attachments
        .entry(self.key) // maybe not exist when entire pool cleared when resize
        .or_default();
      pool.cached.push(self.texture.clone())
    }
  }
}

impl Attachment {
  pub fn des(&self) -> &AttachmentDescriptor {
    &self.des
  }

  pub fn create_default_2d_view(&self) -> GPU2DTextureView {
    self
      .texture
      .create_view(Default::default())
      .try_into()
      .unwrap()
  }

  pub fn write(&mut self) -> AttachmentView<&mut Self> {
    let view = self.create_default_2d_view().into();
    AttachmentView {
      resource: self,
      view,
    }
  }

  pub fn read(&self) -> AttachmentView<&Self> {
    assert_eq!(self.des.sample_count, 1);

    AttachmentView {
      resource: self,
      view: self.create_default_2d_view().into(),
    }
  }

  pub fn read_into(self) -> AttachmentView<Self> {
    assert_eq!(self.des.sample_count, 1);

    let view = self.create_default_2d_view().into();
    AttachmentView {
      resource: self,
      view,
    }
  }
}

pub struct AttachmentView<T> {
  resource: T,
  pub(super) view: RenderTargetView,
}

impl<T> AttachmentView<T> {
  pub fn resource(&self) -> &T {
    &self.resource
  }
}

impl AttachmentView<()> {
  // create the attachment view from any view without ref any resource(T).
  // this is useful to bypass entire static check in attachment ownership and borrow model
  // if user what use external resource directly
  pub fn from_any_view(view: impl Into<RenderTargetView>) -> Self {
    Self {
      resource: Default::default(),
      view: view.into(),
    }
  }
}

impl<T> CacheAbleBindingSource for AttachmentView<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.view.get_binding_build_source()
  }
}

impl<T> rendiation_shader_api::ShaderBindingProvider for AttachmentView<T> {
  type Node = ShaderHandlePtr<ShaderTexture2D>;
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

    let mut resource = ctx.pool.inner.write().unwrap();
    let cached = resource.attachments.entry(key).or_default();

    let texture = cached.cached.pop().unwrap_or_else(|| {
      GPUTexture::create(
        gpu::TextureDescriptor {
          label: None,
          size: map_size_gpu(size),
          dimension: gpu::TextureDimension::D2,
          format: self.format,
          view_formats: &[],
          usage: gpu::TextureUsages::TEXTURE_BINDING
            | gpu::TextureUsages::COPY_DST
            | gpu::TextureUsages::COPY_SRC
            | gpu::TextureUsages::RENDER_ATTACHMENT,
          mip_level_count: 1,
          sample_count: self.sample_count,
        },
        &ctx.gpu.device,
      )
      .try_into()
      .unwrap()
    });

    Attachment {
      pool: ctx.pool.clone(),
      des: self,
      key,
      texture,
    }
  }
}
