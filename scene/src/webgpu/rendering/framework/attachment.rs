use std::{marker::PhantomData, rc::Rc};

use rendiation_texture::Size;
use rendiation_webgpu::{GPUTextureSize, TextureDimension, TextureUsages};

use crate::{RenderEngine, ResourcePool};

pub fn attachment() -> AttachmentDescriptor<wgpu::TextureFormat> {
  AttachmentDescriptor {
    format: wgpu::TextureFormat::Rgba8Unorm,
    sample_count: 1,
    sizer: default_sizer(),
  }
}

pub fn depth_attachment() -> AttachmentDescriptor<wgpu::TextureFormat> {
  AttachmentDescriptor {
    format: wgpu::TextureFormat::Depth24PlusStencil8,
    sample_count: 1,
    sizer: default_sizer(),
  }
}

pub trait AttachmentFormat: Into<wgpu::TextureFormat> + Copy {}
impl<T: Into<wgpu::TextureFormat> + Copy> AttachmentFormat for T {}

#[derive(Clone)]
pub struct Attachment<F: AttachmentFormat> {
  pool: ResourcePool,
  des: AttachmentDescriptor<F>,
  size: Size,
  texture: Option<Rc<wgpu::Texture>>,
}

pub type ColorAttachment = Attachment<wgpu::TextureFormat>;
pub type DepthAttachment = Attachment<wgpu::TextureFormat>; // todo

impl<F: AttachmentFormat> Attachment<F> {
  pub fn write(&mut self) -> AttachmentWriteView<F> {
    AttachmentWriteView {
      phantom: PhantomData,
      size: self.size,
      view: Rc::new(
        self
          .texture
          .as_ref()
          .unwrap()
          .create_view(&wgpu::TextureViewDescriptor::default()),
      ),
      format: self.des.format,
      sample_count: self.des.sample_count,
    }
  }

  pub fn read(&self) -> AttachmentReadView<F> {
    assert_eq!(self.des.sample_count, 1); // todo support latter
    AttachmentReadView {
      phantom: PhantomData,
      view: Rc::new(
        self
          .texture
          .as_ref()
          .unwrap()
          .create_view(&wgpu::TextureViewDescriptor::default()),
      ),
    }
  }

  pub fn read_into(self) -> AttachmentOwnedReadView<F> {
    assert_eq!(self.des.sample_count, 1); // todo support latter
    let view = self
      .texture
      .as_ref()
      .unwrap()
      .create_view(&wgpu::TextureViewDescriptor::default());
    AttachmentOwnedReadView {
      _att: self,
      view: Rc::new(view),
    }
  }
}

impl<F: AttachmentFormat> Drop for Attachment<F> {
  fn drop(&mut self) {
    if let Ok(texture) = Rc::try_unwrap(self.texture.take().unwrap()) {
      let mut pool = self.pool.inner.borrow_mut();
      let cached = pool
        .attachments
        .entry((self.size, self.des.format.into(), self.des.sample_count))
        .or_insert_with(Default::default);

      cached.push(texture)
    }
  }
}

pub struct AttachmentWriteView<'a, F: AttachmentFormat> {
  pub(super) phantom: PhantomData<&'a Attachment<F>>,
  pub(super) size: Size,
  pub(super) view: Rc<wgpu::TextureView>, // todo opt enum
  pub(super) format: F,
  pub(super) sample_count: u32,
}

pub struct AttachmentReadView<'a, F: AttachmentFormat> {
  phantom: PhantomData<&'a Attachment<F>>,
  pub(super) view: Rc<wgpu::TextureView>,
}

// impl<'a, F: AttachmentFormat> BindableResource for AttachmentReadView<'a, F> {
//   fn as_bindable(&self) -> wgpu::BindingResource {
//     wgpu::BindingResource::TextureView(self.view.as_ref())
//   }

//   fn bind_layout() -> wgpu::BindingType {
//     wgpu::BindingType::Texture {
//       multisampled: false,
//       sample_type: wgpu::TextureSampleType::Float { filterable: true },
//       view_dimension: wgpu::TextureViewDimension::D2,
//     }
//   }
// }

pub struct AttachmentOwnedReadView<F: AttachmentFormat> {
  _att: Attachment<F>,
  view: Rc<wgpu::TextureView>,
}

// impl<F: AttachmentFormat> BindableResource for AttachmentOwnedReadView<F> {
//   fn as_bindable(&self) -> wgpu::BindingResource {
//     wgpu::BindingResource::TextureView(self.view.as_ref())
//   }

//   fn bind_layout() -> wgpu::BindingType {
//     wgpu::BindingType::Texture {
//       multisampled: false,
//       sample_type: wgpu::TextureSampleType::Float { filterable: true },
//       view_dimension: wgpu::TextureViewDimension::D2,
//     }
//   }
// }

#[derive(Clone)]
pub struct AttachmentDescriptor<F> {
  pub(super) format: F,
  pub(super) sample_count: u32,
  pub(super) sizer: Rc<dyn Fn(Size) -> Size>,
}

pub fn default_sizer() -> Rc<dyn Fn(Size) -> Size> {
  Rc::new(|size| size)
}

impl<F: AttachmentFormat> AttachmentDescriptor<F> {
  #[must_use]
  pub fn format(mut self, format: F) -> Self {
    self.format = format;
    self
  }
}

impl<F: AttachmentFormat> AttachmentDescriptor<F> {
  pub fn request(self, engine: &RenderEngine) -> Attachment<F> {
    let size = (self.sizer)(engine.output.as_ref().unwrap().size);
    let mut resource = engine.resource.inner.borrow_mut();
    let cached = resource
      .attachments
      .entry((size, self.format.into(), self.sample_count))
      .or_insert_with(Default::default);

    let texture = cached.pop().unwrap_or_else(|| {
      engine.gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: size.into_gpu_size(),
        mip_level_count: 1,
        sample_count: self.sample_count,
        dimension: TextureDimension::D2,
        format: self.format.into(),
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
      })
    });

    Attachment {
      pool: engine.resource.clone(),
      des: self,
      size,
      texture: Rc::new(texture).into(),
    }
  }
}
