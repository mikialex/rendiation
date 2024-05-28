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

pub struct TimeQuery {
  query_set: gpu::QuerySet,
}

impl TimeQuery {
  pub fn start(device: &GPUDevice, encoder: &mut GPUCommandEncoder) -> Self {
    let query_set = device.create_query_set(&QuerySetDescriptor {
      label: "time-query".into(),
      ty: QueryType::Timestamp,
      count: 2,
    });

    encoder.write_timestamp(&query_set, 0);

    Self { query_set }
  }

  /// should use same encoder passed in start fn
  pub fn end(
    self,
    device: &GPUDevice,
    encoder: &mut GPUCommandEncoder,
  ) -> impl Future<Output = Option<TimestampPair>> + Unpin {
    encoder.write_timestamp(&self.query_set, 1);

    read_back_query::<TimestampPair>(&self.query_set, 0..2, device, encoder)
  }
}
