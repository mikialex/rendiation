use crate::*;

#[derive(Default)]
pub struct ResourcePoolImpl {
  attachments: HashMap<PooledTextureKey, SingleResourcePool>,
}

impl ResourcePoolImpl {
  pub fn clear(&mut self) {
    self.attachments.clear();
  }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct PooledTextureKey {
  size: Size,
  format: ::webgpu::TextureFormat,
  sample_count: u32,
}

#[derive(Default)]
struct SingleResourcePool {
  cached: Vec<GPUTexture2d>,
}

#[derive(Clone, Default)]
pub struct ResourcePool {
  pub inner: Rc<RefCell<ResourcePoolImpl>>,
}

impl ResourcePool {
  pub fn clear(&mut self) {
    self.inner.borrow_mut().clear()
  }
}

pub fn attachment() -> AttachmentDescriptor {
  AttachmentDescriptor {
    format: webgpu::TextureFormat::Rgba8Unorm,
    sample_count: 1,
    sizer: default_sizer(),
  }
}

pub fn depth_attachment() -> AttachmentDescriptor {
  AttachmentDescriptor {
    format: webgpu::TextureFormat::Depth24PlusStencil8,
    sample_count: 1,
    sizer: default_sizer(),
  }
}

#[derive(Clone)]
pub struct Attachment {
  pool: ResourcePool,
  des: AttachmentDescriptor,
  texture: GPUTexture2d,
  key: PooledTextureKey,
}

impl Drop for Attachment {
  fn drop(&mut self) {
    let mut pool = self.pool.inner.borrow_mut();
    let pool = pool.attachments.get_mut(&self.key).unwrap();
    pool.cached.push(self.texture.clone())
  }
}

impl Attachment {
  pub fn des(&self) -> &AttachmentDescriptor {
    &self.des
  }

  pub fn write(&mut self) -> AttachmentWriteView<&mut Self> {
    let view = self.texture.create_view(()).into();
    AttachmentWriteView {
      _resource: self,
      view,
    }
  }

  pub fn read(&self) -> AttachmentReadView<&Self> {
    assert_eq!(self.des.sample_count, 1); // todo support latter
    AttachmentReadView {
      _resource: self,
      view: self.texture.create_view(()).into(),
    }
  }

  pub fn read_into(self) -> AttachmentReadView<Self> {
    assert_eq!(self.des.sample_count, 1); // todo support latter
    let view = self.texture.create_view(()).into();
    AttachmentReadView {
      _resource: self,
      view,
    }
  }
}

pub struct AttachmentWriteView<T> {
  _resource: T,
  pub(super) view: RenderTargetView,
}

pub struct AttachmentReadView<T> {
  _resource: T,
  pub(super) view: RenderTargetView,
}

impl<T> BindingSource for AttachmentReadView<T> {
  type Uniform = RenderTargetView;

  fn get_uniform(&self) -> Self::Uniform {
    self.view.clone()
  }
}

impl<T> ShaderUniformProvider for AttachmentReadView<T> {
  type Node = ShaderTexture2D;
}

#[derive(Clone)]
pub struct AttachmentDescriptor {
  pub(super) format: webgpu::TextureFormat,
  pub(super) sample_count: u32,
  pub(super) sizer: Rc<dyn Fn(Size) -> Size>,
}

pub fn default_sizer() -> Rc<dyn Fn(Size) -> Size> {
  Rc::new(|size| size)
}

impl AttachmentDescriptor {
  #[must_use]
  pub fn format(mut self, format: webgpu::TextureFormat) -> Self {
    self.format = format;
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

    let mut resource = ctx.pool.inner.borrow_mut();
    let cached = resource
      .attachments
      .entry(key)
      .or_insert_with(Default::default);

    let texture = cached.cached.pop().unwrap_or_else(|| {
      GPUTexture2d::create(
        WebGPUTexture2dDescriptor::from_size(size)
          .with_render_target_ability()
          .with_sample_count(self.sample_count)
          .with_format(self.format),
        &ctx.gpu.device,
      )
    });

    Attachment {
      pool: ctx.pool.clone(),
      des: self,
      key,
      texture,
    }
  }
}
