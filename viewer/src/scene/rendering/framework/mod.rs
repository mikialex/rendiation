pub mod pass;

pub use pass::*;

pub mod pool;
pub use pool::*;

pub mod attachment;
pub use attachment::*;

use interphaser::FrameTarget;
use rendiation_webgpu::GPU;
use std::{marker::PhantomData, rc::Rc};

pub struct RenderEngine {
  resource: ResourcePool,
  gpu: Rc<GPU>,
  msaa_sample_count: u32,
  pub output: Option<FrameTarget>,
}

impl RenderEngine {
  pub fn new(gpu: Rc<GPU>) -> Self {
    Self {
      resource: Default::default(),
      output: Default::default(),
      msaa_sample_count: 4,
      gpu,
    }
  }

  pub fn notify_output_resized(&self) {
    self.resource.inner.borrow_mut().attachments.clear();
  }

  pub fn screen(&self) -> AttachmentWriteView<wgpu::TextureFormat> {
    let output = self.output.as_ref().unwrap();
    AttachmentWriteView {
      phantom: PhantomData,
      size: output.size,
      view: output.view.clone(),
      format: output.format,
      sample_count: 1,
    }
  }

  pub fn multisampled_attachment(&self) -> AttachmentDescriptor<wgpu::TextureFormat> {
    AttachmentDescriptor {
      format: wgpu::TextureFormat::Rgba8Unorm,
      sample_count: self.msaa_sample_count,
      sizer: default_sizer(),
    }
  }

  pub fn multisampled_depth_attachment(&self) -> AttachmentDescriptor<wgpu::TextureFormat> {
    AttachmentDescriptor {
      format: wgpu::TextureFormat::Depth24PlusStencil8,
      sample_count: self.msaa_sample_count,
      sizer: default_sizer(),
    }
  }
}
