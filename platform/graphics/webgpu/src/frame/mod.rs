mod pass;
use std::mem::ManuallyDrop;

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
  pub encoder: ManuallyDrop<GPUCommandEncoder>,
  pool: &'a AttachmentPool,
  statistics: Option<FrameStaticInfoResolver>,
  frame_size: Size,
  frame_index: u64,
}

impl Drop for FrameCtx<'_> {
  fn drop(&mut self) {
    if let Some(stat) = &mut self.statistics {
      stat.resolve(self.gpu, &mut self.encoder);
    }
    let encoder = unsafe { ManuallyDrop::take(&mut self.encoder) };
    self.gpu.submit_encoder(encoder)
  }
}

impl<'a> FrameCtx<'a> {
  pub fn new(gpu: &'a GPU, frame_size: Size, pool: &'a AttachmentPool, frame_index: u64) -> Self {
    let encoder = ManuallyDrop::new(gpu.create_encoder());

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
    let mut encoder = ManuallyDrop::new(self.gpu.create_encoder());
    std::mem::swap(&mut self.encoder, &mut encoder);
    self.gpu.submit_encoder(ManuallyDrop::into_inner(encoder))
  }

  pub fn frame_size(&self) -> Size {
    self.frame_size
  }
}
