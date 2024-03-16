use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::SeqCst;

mod counter;
pub use counter::*;

mod allocator_hook;
pub use allocator_hook::*;

#[derive(Default)]
struct CounterRecord {
  pub current: AtomicU64,
  pub history_peak: AtomicU64,
}

impl CounterRecord {
  // default impl is not const
  pub const fn new() -> Self {
    Self {
      current: AtomicU64::new(0),
      history_peak: AtomicU64::new(0),
    }
  }

  #[allow(unused)]
  pub fn increase(&self, count: u64) {
    let pre_current = self.current.fetch_add(count, SeqCst);
    self.history_peak.fetch_max(pre_current + count, SeqCst);
  }
  #[allow(unused)]
  pub fn decrease(&self, count: u64) {
    self.current.fetch_sub(count, SeqCst);
  }

  pub fn reset_history_peak_to_current(&self) {
    let current = self.current.fetch_add(0, SeqCst);
    self.history_peak.fetch_min(current, SeqCst);
  }

  pub fn report(&self) -> CounterRecordReport<u64> {
    CounterRecordReport {
      current: self.current.fetch_add(0, SeqCst),
      history_peak: self.history_peak.fetch_add(0, SeqCst),
    }
  }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct CounterRecordReport<T> {
  pub current: T,
  pub history_peak: T,
}

impl<T> CounterRecordReport<T> {
  pub fn map<U>(self, f: impl Fn(T) -> U) -> CounterRecordReport<U> {
    CounterRecordReport {
      current: f(self.current),
      history_peak: f(self.history_peak),
    }
  }
}
