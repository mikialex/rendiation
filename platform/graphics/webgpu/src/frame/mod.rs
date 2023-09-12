pub mod pass;
pub use pass::*;

pub mod attachment;
pub use attachment::*;

use crate::*;

pub struct FrameCtx<'a> {
  pub gpu: &'a GPU,
  pool: &'a ResourcePool,
  msaa_sample_count: u32,
  frame_size: Size,
  pub encoder: GPUCommandEncoder,
}

impl<'a> FrameCtx<'a> {
  pub fn new(gpu: &'a GPU, frame_size: Size, pool: &'a ResourcePool) -> Self {
    let msaa_sample_count = 4;

    let encoder = gpu.create_encoder();

    Self {
      pool,
      frame_size,
      msaa_sample_count,
      encoder,
      gpu,
    }
  }

  pub fn make_submit(&mut self) {
    let mut encoder = self.gpu.create_encoder();
    std::mem::swap(&mut self.encoder, &mut encoder);
    self.gpu.submit_encoder(encoder)
  }

  pub fn final_submit(self) {
    self.gpu.submit_encoder(self.encoder)
  }

  pub fn multisampled_attachment(&self) -> AttachmentDescriptor {
    AttachmentDescriptor {
      format: gpu::TextureFormat::Rgba8UnormSrgb,
      sample_count: self.msaa_sample_count,
      sizer: default_sizer(),
    }
  }

  pub fn multisampled_depth_attachment(&self) -> AttachmentDescriptor {
    AttachmentDescriptor {
      format: gpu::TextureFormat::Depth24PlusStencil8,
      sample_count: self.msaa_sample_count,
      sizer: default_sizer(),
    }
  }
}
