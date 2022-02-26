pub mod pass;
pub use pass::*;

pub mod attachment;
pub use attachment::*;

use rendiation_webgpu::*;
use std::{marker::PhantomData, rc::Rc};

pub struct RenderEngine {
  resource: ResourcePool,
  msaa_sample_count: u32,
  gpu: Rc<GPU>,
  encoder: GPUCommandEncoder,
  pub output: ColorChannelView,
}

impl RenderEngine {
  pub fn new(gpu: Rc<GPU>, output: ColorChannelView) -> Self {
    #[allow(unused_mut)]
    let mut msaa_sample_count = 4;

    #[cfg(all(target_arch = "wasm32", feature = "webgl"))]
    {
      msaa_sample_count = 1;
    }

    let encoder = gpu.create_encoder();

    Self {
      resource: Default::default(),
      output,
      msaa_sample_count,
      encoder,
      gpu,
    }
  }

  pub fn notify_output_resized(&self) {
    self.resource.inner.borrow_mut().attachments.clear();
  }

  pub fn screen(&self) -> AttachmentWriteView {
    AttachmentWriteView {
      phantom: PhantomData,
      view: self.output.clone(),
    }
  }

  pub fn multisampled_attachment(&self) -> AttachmentDescriptor {
    AttachmentDescriptor {
      format: wgpu::TextureFormat::Rgba8Unorm,
      sample_count: self.msaa_sample_count,
      sizer: default_sizer(),
    }
  }

  pub fn multisampled_depth_attachment(&self) -> AttachmentDescriptor {
    AttachmentDescriptor {
      format: wgpu::TextureFormat::Depth24PlusStencil8,
      sample_count: self.msaa_sample_count,
      sizer: default_sizer(),
    }
  }
}
