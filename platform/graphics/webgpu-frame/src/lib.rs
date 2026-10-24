use std::mem::ManuallyDrop;
use std::sync::Arc;

use fast_hash_collection::*;
use hook::*;
use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_shader_library::*;
use rendiation_webgpu::*;
use reuse_pool::*;

mod pass;
pub use pass::*;

mod attachment;
pub use attachment::*;

mod statistics;
pub use statistics::*;

mod full_screen;
pub use full_screen::*;

mod pass_base;
pub use pass_base::*;

mod pass_info;
pub use pass_info::*;

pub struct FrameCtx<'a> {
  pub gpu: &'a GPU,
  /// note, wrap in manually drop enable us do submitting in drop fn
  pub encoder: ManuallyDrop<GPUCommandEncoder>,
  pool: &'a AttachmentPool,
  pass_info_pool: &'a PassInfoPool,
  statistics: Option<FramePassStatisticsRecorder>,
  /// if we are in the ctx of viewport, the frame size is the viewport frame size
  pub frame_size: Size,
  pub memory: &'a mut FunctionMemory,
}

unsafe impl HooksCxLike for FrameCtx<'_> {
  fn memory_mut(&mut self) -> &mut FunctionMemory {
    self.memory
  }

  fn memory_ref(&self) -> &FunctionMemory {
    self.memory
  }

  fn flush(&mut self) {
    let drop_cx = &mut () as *mut ();
    self.memory.flush(drop_cx);
  }

  fn is_dynamic_stage(&self) -> bool {
    true
  }

  fn use_plain_state<T: 'static>(&mut self, f: impl FnOnce() -> T) -> (&mut Self, &mut T) {
    let this = self as *mut Self;
    let state = unsafe {
      (*this)
        .memory
        .expect_state_init(f, |_state: &mut T, _: &mut ()| {})
    };
    // SAFETY: this is derived from a valid &mut self; state points into bump-allocated heap
    // memory inside memory, not into the struct itself, so no aliased &mut is created.
    let this = unsafe { &mut *this };
    (this, state)
  }
}

impl Drop for FrameCtx<'_> {
  fn drop(&mut self) {
    // the encoder may be in a broken state when panicking, submitting it may cause another
    // panic which hides the original one.
    if std::thread::panicking() {
      unsafe { ManuallyDrop::drop(&mut self.encoder) };
      return;
    }

    if let Some(statistics) = &mut self.statistics {
      statistics.resolve(&self.gpu.device, &mut self.encoder);
    }

    let encoder = unsafe { ManuallyDrop::take(&mut self.encoder) };
    self.gpu.submit_encoder(encoder)
  }
}

impl<'a> FrameCtx<'a> {
  pub fn new(
    gpu: &'a GPU,
    memory: &'a mut FunctionMemory,
    frame_size: Size,
    pool: &'a AttachmentPool,
    pass_info_pool: &'a PassInfoPool,
    statistics: Option<FramePassStatisticsRecorder>,
  ) -> Self {
    let encoder = ManuallyDrop::new(gpu.create_encoder());

    Self {
      pool,
      frame_size,
      statistics,
      encoder,
      gpu,
      pass_info_pool,
      memory,
    }
  }

  pub fn frame_size(&self) -> Size {
    self.frame_size
  }
}
