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

mod pass_info;
pub use pass_info::*;

use crate::*;

pub struct FrameCtx<'a> {
  pub gpu: &'a GPU,
  /// note, wrap in manually drop enable us do submitting in drop fn
  pub encoder: ManuallyDrop<GPUCommandEncoder>,
  pool: &'a AttachmentPool,
  /// currently we recreate pool every frame, this can be improved
  /// to avoid unnecessary bindgroup invalidation.
  pass_info_pool: &'a PassInfoPool,
  statistics: Option<FrameStaticInfoResolver>,
  pub frame_size: Size,
}

impl Drop for FrameCtx<'_> {
  fn drop(&mut self) {
    if let Some(statistics) = &mut self.statistics {
      statistics.resolve(self.gpu, &mut self.encoder);
    }

    let encoder = unsafe { ManuallyDrop::take(&mut self.encoder) };
    self.gpu.submit_encoder(encoder)
  }
}

impl<'a> FrameCtx<'a> {
  pub fn new(
    gpu: &'a GPU,
    frame_size: Size,
    pool: &'a AttachmentPool,
    pass_info_pool: &'a PassInfoPool,
    statistics: Option<FrameStaticInfoResolver>,
  ) -> Self {
    let encoder = ManuallyDrop::new(gpu.create_encoder());

    Self {
      pool,
      frame_size,
      statistics,
      encoder,
      gpu,
      pass_info_pool,
    }
  }

  pub fn frame_size(&self) -> Size {
    self.frame_size
  }
}
