use std::num::NonZeroU64;

use futures::{Future, FutureExt};

use crate::*;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TimestampPair {
  pub start: u64,
  pub end: u64,
}

impl TimestampPair {
  pub fn duration_in_ms(&self, time_unit_in_nanoseconds: f32) -> f64 {
    let nanoseconds = (self.end - self.start) as f64 * time_unit_in_nanoseconds as f64;
    nanoseconds / 1_000_000.0
  }
}

impl GPUCommandEncoder {
  pub fn measure_gpu_time(
    &mut self,
    device: &GPUDevice,
    scope: impl FnOnce(&mut GPUCommandEncoder),
  ) -> impl Future<Output = Option<TimestampPair>> + Unpin {
    let query = TimeQuery::new(device);

    self.write_timestamp(&query.query_set, 0);

    scope(self);

    self.write_timestamp(&query.query_set, 1);
    self.resolve_query_set(&query.query_set, 0..2, query.result.gpu(), 0);

    self
      .read_buffer(device, &query.result.create_default_view())
      .map(|r| {
        r.ok().map(|r| {
          let view = &r.read_raw()[0..std::mem::size_of::<TimestampPair>()];
          *bytemuck::from_bytes(view)
        })
      })
  }
}

struct TimeQuery {
  query_set: gpu::QuerySet,
  result: GPUBufferResource,
}

impl TimeQuery {
  pub fn new(device: &GPUDevice) -> Self {
    let query_set = device.create_query_set(&QuerySetDescriptor {
      label: "time-query".into(),
      ty: QueryType::Timestamp,
      count: 2,
    });

    let size = std::mem::size_of::<TimestampPair>().max(QUERY_RESOLVE_BUFFER_ALIGNMENT as usize);
    let size = NonZeroU64::new(size as u64).unwrap();

    let init = BufferInit::Zeroed(size);
    let usage = BufferUsages::COPY_SRC | BufferUsages::QUERY_RESOLVE;
    let desc = GPUBufferDescriptor {
      size: init.size(),
      usage,
    };

    let buffer = GPUBuffer::create(device, init, usage);
    let result = GPUBufferResource::create_with_raw(buffer, desc, device);

    Self { query_set, result }
  }
}
