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
  cached: Vec<GPU2DTexture>,
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
    format: webgpu::TextureFormat::Depth32Float,
    sample_count: 1,
    sizer: default_sizer(),
  }
}

pub fn depth_stencil_attachment() -> AttachmentDescriptor {
  AttachmentDescriptor {
    format: webgpu::TextureFormat::Depth24PlusStencil8,
    sample_count: 1,
    sizer: default_sizer(),
  }
}

/// Ownership is always transferred, do not support clone.
pub struct Attachment {
  pool: ResourcePool,
  des: AttachmentDescriptor,
  texture: GPU2DTexture,
  key: PooledTextureKey,
}

impl AsRef<Attachment> for Attachment {
  fn as_ref(&self) -> &Attachment {
    self
  }
}

/// When it drops, return the texture to the reusing pool;
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

  fn create_default_2d_view(&self) -> GPU2DTextureView {
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
    assert_eq!(self.des.sample_count, 1); // todo support latter

    AttachmentView {
      resource: self,
      view: self.create_default_2d_view().into(),
    }
  }

  pub fn read_into(self) -> AttachmentView<Self> {
    assert_eq!(self.des.sample_count, 1); // todo support latter

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

impl<T> BindingSource for AttachmentView<T> {
  type Uniform = RenderTargetView;

  fn get_uniform(&self) -> Self::Uniform {
    self.view.clone()
  }
}

impl<T> ShaderUniformProvider for AttachmentView<T> {
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
      GPUTexture::create(
        webgpu::TextureDescriptor {
          label: None,
          size: map_size_gpu(size),
          dimension: webgpu::TextureDimension::D2,
          format: self.format,
          usage: webgpu::TextureUsages::TEXTURE_BINDING
            | webgpu::TextureUsages::COPY_DST
            | webgpu::TextureUsages::RENDER_ATTACHMENT,
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
