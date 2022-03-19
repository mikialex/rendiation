pub mod pass;
pub use pass::*;

pub mod attachment;
pub use attachment::*;

use rendiation_webgpu::*;

use crate::GPUResourceCache;

pub struct FrameCtx<'a> {
  pool: &'a ResourcePool,
  msaa_sample_count: u32,
  gpu: &'a GPU,
  encoder: GPUCommandEncoder,
  resources: &'a mut GPUResourceCache,
  pub output: ColorChannelView,
}

impl<'a> FrameCtx<'a> {
  pub fn new(
    gpu: &'a GPU,
    output: ColorChannelView,
    pool: &'a ResourcePool,
    resources: &'a mut GPUResourceCache,
  ) -> Self {
    let msaa_sample_count = 4;

    let encoder = gpu.create_encoder();

    Self {
      pool,
      output,
      resources,
      msaa_sample_count,
      encoder,
      gpu,
    }
  }

  pub fn notify_output_resized(&self) {
    self.pool.inner.borrow_mut().clear();
  }

  pub fn screen(&self) -> AttachmentWriteView<&mut Attachment> {
    AttachmentWriteView {
      resource: todo!(),
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
