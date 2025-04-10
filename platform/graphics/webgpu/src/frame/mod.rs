mod pass;
pub use pass::*;

mod attachment;
pub use attachment::*;

mod statistics;
pub use statistics::*;

mod quad;
pub use quad::*;

mod pass_base;
pub use pass_base::*;

use crate::*;

pub struct FrameCtx<'a> {
  pub gpu: &'a GPU,
  pub encoder: GPUCommandEncoder,
  pool: &'a AttachmentPool,
  statistics: Option<StatisticTaskSender>,
  frame_size: Size,
  frame_index: u64,
}

impl<'a> FrameCtx<'a> {
  pub fn new(gpu: &'a GPU, frame_size: Size, pool: &'a AttachmentPool, frame_index: u64) -> Self {
    let encoder = gpu.create_encoder();

    Self {
      pool,
      frame_size,
      frame_index,
      statistics: None,
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

  pub fn frame_size(&self) -> Size {
    self.frame_size
  }
}
