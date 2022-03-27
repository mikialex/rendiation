use std::{cell::RefCell, collections::HashMap, rc::Rc};

use rendiation_texture::*;
use rendiation_webgpu::*;
use shadergraph::{ShaderTexture, ShaderUniformProvider};

use crate::FrameCtx;

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
  format: wgpu::TextureFormat,
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
    format: wgpu::TextureFormat::Rgba8Unorm,
    sample_count: 1,
    sizer: default_sizer(),
  }
}

pub fn depth_attachment() -> AttachmentDescriptor {
  AttachmentDescriptor {
    format: wgpu::TextureFormat::Depth24PlusStencil8,
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
  pub fn write(&mut self) -> AttachmentWriteView<&mut Self> {
    let view = self.texture.create_view(()).into();
    AttachmentWriteView {
      resource: self,
      view,
    }
  }

  pub fn read(&self) -> AttachmentReadView<&Self> {
    assert_eq!(self.des.sample_count, 1); // todo support latter
    AttachmentReadView {
      resource: self,
      view: self.texture.create_view(()).into(),
    }
  }

  pub fn read_into(self) -> AttachmentReadView<Self> {
    assert_eq!(self.des.sample_count, 1); // todo support latter
    let view = self.texture.create_view(()).into();
    AttachmentReadView {
      resource: self,
      view,
    }
  }
}

pub struct AttachmentWriteView<T> {
  pub(super) resource: T,
  pub(super) view: ColorChannelView,
}

pub struct AttachmentReadView<T> {
  resource: T,
  pub(super) view: ColorChannelView,
}

impl<T> BindingSource for AttachmentReadView<T> {
  type Uniform = GPUTexture2dView;

  fn get_uniform(&self) -> Self::Uniform {
    todo!()
  }
}

impl<T> ShaderUniformProvider for AttachmentReadView<T> {
  type Node = ShaderTexture;
}

#[derive(Clone)]
pub struct AttachmentDescriptor {
  pub(super) format: wgpu::TextureFormat,
  pub(super) sample_count: u32,
  pub(super) sizer: Rc<dyn Fn(Size) -> Size>,
}

pub fn default_sizer() -> Rc<dyn Fn(Size) -> Size> {
  Rc::new(|size| size)
}

impl AttachmentDescriptor {
  #[must_use]
  pub fn format(mut self, format: wgpu::TextureFormat) -> Self {
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
