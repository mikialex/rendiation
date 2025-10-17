use std::alloc::GlobalAlloc;

use crate::*;

#[derive(Clone, Copy, Default)]
pub struct ReadableByteDisplay(pub u64);
impl std::fmt::Debug for ReadableByteDisplay {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let size = humansize::format_size(self.0, humansize::BINARY);
    f.write_str(&size)
  }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PreciseAllocationReport {
  pub allocation_real_bytes_count: CounterRecordReport<ReadableByteDisplay>,
  pub allocation_instance_count: CounterRecordReport<u64>,
  pub allocation_event_count: u64,
}

pub struct PreciseAllocationStatistics<T> {
  pub allocator: T,
  allocation_real_bytes_count: CounterRecord,
  allocation_instance_count: CounterRecord,
  allocation_event_count: AtomicU64,
}

impl<T> PreciseAllocationStatistics<T> {
  pub const fn new(allocator: T) -> Self {
    Self {
      allocator,
      allocation_real_bytes_count: CounterRecord::new(),
      allocation_instance_count: CounterRecord::new(),
      allocation_event_count: AtomicU64::new(0),
    }
  }

  pub fn reset_history_peak(&self) {
    self
      .allocation_real_bytes_count
      .reset_history_peak_to_current();
    self
      .allocation_instance_count
      .reset_history_peak_to_current();
  }

  pub fn reset_allocation_event_counter(&self) {
    self.allocation_event_count.store(0, SeqCst);
  }

  pub fn report(&self) -> PreciseAllocationReport {
    PreciseAllocationReport {
      allocation_real_bytes_count: self
        .allocation_real_bytes_count
        .report()
        .map(ReadableByteDisplay),
      allocation_instance_count: self.allocation_instance_count.report(),
      allocation_event_count: self.allocation_event_count.load(SeqCst),
    }
  }
}

unsafe impl<T: GlobalAlloc> GlobalAlloc for PreciseAllocationStatistics<T> {
  unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
    #[cfg(feature = "enabled")]
    {
      self.allocation_instance_count.increase(1);
      self
        .allocation_real_bytes_count
        .increase(layout.size() as u64);
    }
    self.allocation_event_count.fetch_add(1, SeqCst);

    self.allocator.alloc(layout)
  }

  unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
    #[cfg(feature = "enabled")]
    {
      self.allocation_instance_count.decrease(1);
      self
        .allocation_real_bytes_count
        .decrease(layout.size() as u64);
    }

    self.allocator.dealloc(ptr, layout)
  }
}
