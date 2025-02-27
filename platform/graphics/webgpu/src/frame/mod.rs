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
  pub encoder: GPUCommandEncoder,
  pool: &'a AttachmentPool,
  pub buffer_pool: &'a TempBufferReusePool,
  frame_size: Size,
}

impl<'a> FrameCtx<'a> {
  pub fn new(
    gpu: &'a GPU,
    frame_size: Size,
    pool: &'a AttachmentPool,
    buffer_pool: &'a TempBufferReusePool,
  ) -> Self {
    let encoder = gpu.create_encoder();

    Self {
      pool,
      buffer_pool,
      frame_size,
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

pub type TempBufferReusePool = ReuseKVPool<u32, GPUBufferResourceView>;
pub fn init_temp_buffer_reuse_pool(gpu: &GPU) -> TempBufferReusePool {
  ReuseKVPool::new(|byte_size| {
    //
    todo!()
  })
}
