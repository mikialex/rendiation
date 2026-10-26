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

    if let Some(statistics) = self.statistics.take() {
      statistics.finish(&mut self.encoder);
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

#[cfg(test)]
mod test {
  use std::task::Context;
  use std::time::{Duration, Instant};

  use super::*;

  /// the passes of a [FrameCtx] are recorded by its statistics recorder, and the frame finished
  /// when the ctx drops
  #[test]
  fn test_frame_ctx_pass_statistics() {
    let gpu = pollster::block_on(GPU::new(Default::default())).unwrap();
    let mut statistics = FramePassStatistics::new(4, &gpu);
    let pool = init_attachment_pool(&gpu);
    let pass_info_pool = PassInfoPool::default();
    let mut memory = FunctionMemory::default();

    let frame_index = 1;
    {
      let recorder = statistics.create_frame_recorder(frame_index);
      let size = Size::from_u32_pair_min_one((4, 4));
      let mut ctx = FrameCtx::new(
        &gpu,
        &mut memory,
        size,
        &pool,
        &pass_info_pool,
        Some(recorder),
      );
      let target = attachment().request(&ctx);
      for name in ["a", "b", "a"] {
        let _ = pass(name)
          .with_color(&target, store_full_frame())
          .render_ctx(&mut ctx);
      }
    }

    let any_query_supported =
      statistics.time_query_supported || statistics.pipeline_query_supported;
    if !any_query_supported {
      return;
    }

    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    let start = Instant::now();
    while statistics.pending_frame_count() > 0 && start.elapsed() < Duration::from_secs(5) {
      statistics.poll(&mut cx);
      std::thread::sleep(Duration::from_millis(1));
    }

    assert_eq!(statistics.collected.len(), 2);
    for name in ["a", "b"] {
      let s = &statistics.collected[name];
      if statistics.time_query_supported {
        let (time, index) = s.time.get_latest().unwrap();
        assert_eq!(*index, frame_index);
        assert!(*time >= 0.);
      }
      if statistics.pipeline_query_supported {
        let (_, index) = s.pipeline.get_latest().unwrap();
        assert_eq!(*index, frame_index);
      }
    }
  }
}
