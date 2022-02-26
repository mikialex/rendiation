use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use rendiation_texture::{Size, Texture2D};
use rendiation_webgpu::{
  ColorChannelView, GPUTexture2d, GPUTexture2dView, GPUTextureSize, TextureDimension, TextureUsages,
};

use crate::RenderEngine;

#[derive(Default)]
pub struct ResourcePoolImpl {
  pub attachments: HashMap<(Size, wgpu::TextureFormat, u32), Vec<GPUTexture2d>>,
}

#[derive(Clone, Default)]
pub struct ResourcePool {
  pub inner: Rc<RefCell<ResourcePoolImpl>>,
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
  size: Size,
  texture: GPUTexture2d,
}

impl Attachment {
  pub fn write(&mut self) -> AttachmentWriteView {
    AttachmentWriteView {
      phantom: PhantomData,
      view: self.texture.create_view(Default::default()).into(),
    }
  }

  pub fn read(&self) -> AttachmentReadView {
    assert_eq!(self.des.sample_count, 1); // todo support latter
    AttachmentReadView {
      phantom: PhantomData,
      view: self.texture.create_view(Default::default()).into(),
    }
  }

  pub fn read_into(self) -> AttachmentOwnedReadView {
    assert_eq!(self.des.sample_count, 1); // todo support latter
    let view = self.texture.create_view(()).into();
    AttachmentOwnedReadView { _att: self, view }
  }
}

pub struct AttachmentWriteView<'a> {
  pub(super) phantom: PhantomData<&'a Attachment>,
  pub(super) view: ColorChannelView,
}

pub struct AttachmentReadView<'a> {
  phantom: PhantomData<&'a Attachment>,
  pub(super) view: ColorChannelView,
}

pub struct AttachmentOwnedReadView {
  _att: Attachment,
  view: ColorChannelView,
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
  pub fn request(self, engine: &RenderEngine) -> Attachment {
    let size = match &engine.output {
      ColorChannelView::Texture(t) => t.resource.desc.size,
      ColorChannelView::SurfaceTexture(t) => todo!(),
    };
    let size = GPUTextureSize::from_gpu_size(size);
    let size = (self.sizer)(size);
    let mut resource = engine.resource.inner.borrow_mut();
    let cached = resource
      .attachments
      .entry((size, self.format.into(), self.sample_count))
      .or_insert_with(Default::default);

    // todo check ref count and find available resource
    todo!();

    let texture = cached.pop().unwrap_or_else(|| {
      // GPUTexture2d::create(GPUTexture2dDescriptor::default(), device)
      // engine.gpu.device.create_texture(&wgpu::TextureDescriptor {
      //   label: None,
      //   size: size.into_gpu_size(),
      //   mip_level_count: 1,
      //   sample_count: self.sample_count,
      //   dimension: TextureDimension::D2,
      //   format: self.format.into(),
      //   usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
      // })

      todo!()
    });

    Attachment {
      pool: engine.resource.clone(),
      des: self,
      size,
      texture: texture.clone(),
    }
  }
}
