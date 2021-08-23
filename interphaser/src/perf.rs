use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct PerformanceInfo {
  pub frame_id: usize,
  pub update_time: Duration,
  pub layout_time: Duration,
  pub rendering_prepare_time: Duration,
  pub rendering_dispatch_time: Duration,
  pub all_time: Duration,
}

impl PerformanceInfo {
  pub fn new(frame_id: usize) -> Self {
    Self {
      frame_id,
      update_time: Default::default(),
      layout_time: Default::default(),
      rendering_prepare_time: Default::default(),
      rendering_dispatch_time: Default::default(),
      all_time: Default::default(),
    }
  }
}

pub fn time_measure(f: impl FnOnce()) -> Duration {
  let time = Instant::now();
  f();
  time.elapsed()
}
