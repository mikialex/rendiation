use crate::*;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TimestampPair {
  pub start: u64,
  pub end: u64,
}

impl TimestampPair {
  pub fn duration_in_ms(&self, time_unit_in_nanoseconds: f32) -> f64 {
    // prevent some platform edge case
    if self.start > self.end {
      return f64::NAN;
    }
    let nanoseconds = (self.end - self.start) as f64 * time_unit_in_nanoseconds as f64;
    nanoseconds / 1_000_000.0
  }
}

pub struct TimeQuery {
  pub(crate) query_set: gpu::QuerySet,
}

impl TimeQuery {
  pub fn create_query_without_start(device: &GPUDevice) -> Self {
    let query_set = device.create_query_set(&QuerySetDescriptor {
      label: "time-query".into(),
      ty: QueryType::Timestamp,
      count: 2,
    });
    Self { query_set }
  }

  pub fn create_and_start_directly_from_encoder(
    device: &GPUDevice,
    encoder: &mut GPUCommandEncoder,
  ) -> Self {
    let t = Self::create_query_without_start(device);
    t.start_directly_from_encoder(encoder);
    t
  }

  pub fn start_directly_from_encoder(&self, encoder: &mut GPUCommandEncoder) {
    encoder.write_timestamp(&self.query_set, 0);
  }

  /// should use same encoder passed in start fn
  pub fn end_directly_from_encoder(
    self,
    device: &GPUDevice,
    encoder: &mut GPUCommandEncoder,
  ) -> impl Future<Output = Option<TimestampPair>> + Unpin {
    encoder.write_timestamp(&self.query_set, 1);
    self.read_back(device, encoder)
  }

  pub fn read_back(
    self,
    device: &GPUDevice,
    encoder: &mut GPUCommandEncoder,
  ) -> impl Future<Output = Option<TimestampPair>> + Unpin {
    read_back_query::<TimestampPair>(&self.query_set, 0..2, device, encoder)
  }
}
