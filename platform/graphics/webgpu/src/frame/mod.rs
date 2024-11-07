mod pass;
pub use pass::*;

mod attachment;
pub use attachment::*;

mod quad;
pub use quad::*;

mod pass_base;
pub use pass_base::*;

use crate::*;

pub struct FrameCtx<'a> {
  pub gpu: &'a GPU,
  pub render_encoder: GPUCommandEncoder,
  pool: &'a AttachmentPool,
  frame_size: Size,
}

impl<'a> FrameCtx<'a> {
  pub fn new(gpu: &'a GPU, frame_size: Size, pool: &'a AttachmentPool) -> Self {
    let encoder = gpu.create_encoder();

    Self {
      pool,
      frame_size,
      render_encoder: encoder,
      gpu,
    }
  }

  pub fn make_submit(&mut self) {
    let mut encoder = self.gpu.create_encoder();
    std::mem::swap(&mut self.render_encoder, &mut encoder);
    self.gpu.submit_encoder(encoder)
  }

  pub fn final_submit(self) {
    self.gpu.submit_encoder(self.render_encoder)
  }

  pub fn frame_size(&self) -> Size {
    self.frame_size
  }
}
