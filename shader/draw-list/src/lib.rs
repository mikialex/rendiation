use dyn_clone::DynClone;
use rendiation_device_parallel_compute::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod device_culling;
mod list_access;
mod multi_range;
mod stream_compact;
pub use device_culling::*;
pub use multi_range::*;

#[derive(Clone)]
pub struct DeviceDrawList {
  pub id_pool: StorageBufferReadonlyDataView<[u32]>,
  pub dispatch_info: MultiRangeDispatchInfo,
}

#[derive(Clone)]
pub struct MultiRangeDispatchInfo {
  pub device_ranges: DeviceMultiRangeDispatchInfo,
  pub host_capacity_ranges: Vec<CapacityRange>,
  pub total_capacity: u32,
}

#[derive(Clone)]
pub struct CapacityRange {
  /// must not equal to zero
  pub capacity: u32,
  pub offset: u32,
}

pub fn prepare_gpu_sub_list_ranges(
  host_capacity_ranges: &[CapacityRange],
  real_length: &[u32],
) -> Vec<StorageSubListRangeInfo> {
  assert_eq!(host_capacity_ranges.len(), real_length.len());
  let sub_count = host_capacity_ranges.len();
  let mut prefix_sum = 0u32;
  let mut ranges = Vec::with_capacity(sub_count);
  for (info, &length) in host_capacity_ranges.iter().zip(real_length.iter()) {
    assert!(info.capacity >= length);

    ranges.push(StorageSubListRangeInfo::new(
      info.offset,
      length,
      prefix_sum,
    ));
    prefix_sum += length;
  }
  ranges
}

impl DeviceDrawList {
  /// Creates (or reuses from cache) a DeviceDrawList with pre-allocated output buffers sized
  /// according to per-sub-list capacities. The sub_list_ranges are initialized with zero counts;
  /// the GPU fills in actual survival counts during culling.
  pub(crate) fn create_or_update_compact_culling_write_target<'a>(
    &self,
    gpu: &GPU,
    cached: &'a mut Option<Self>,
    sub_list_infos: &[CapacityRange],
  ) -> &'a Self {
    // we do not do any storage buffer binding alignment here because
    // we assume the input list's offset has correctly aligned and capacity has round up
    let total_capacity: u32 = sub_list_infos.iter().map(|info| info.capacity).sum();

    // Reuse cached target if the total capacity matches.
    let needs_create = match cached.as_ref() {
      Some(existing) => existing.dispatch_info.total_capacity != total_capacity,
      None => true,
    };

    // the real count and offsets are override by compute shader write
    let length = vec![0_u32; sub_list_infos.len()];
    let ranges_init = prepare_gpu_sub_list_ranges(sub_list_infos, &length);
    if needs_create {
      let device_ranges = DeviceMultiRangeDispatchInfo::new(gpu, ranges_init.as_slice());

      let pool_data = vec![0u32; total_capacity as usize];
      let scene_model_id_pool = create_gpu_readonly_storage(
        pool_data.as_slice(),
        gpu,
        "device draw list scene_model_id_pool",
      );

      *cached = Some(DeviceDrawList {
        id_pool: scene_model_id_pool,
        dispatch_info: MultiRangeDispatchInfo {
          device_ranges,
          host_capacity_ranges: sub_list_infos.to_vec(),
          total_capacity,
        },
      });
    } else {
      let target = cached.as_ref().unwrap();
      target
        .dispatch_info
        .device_ranges
        .update(gpu, ranges_init.as_slice());
    }

    cached.as_ref().unwrap()
  }

  pub fn create_indirect_count_views(&self) -> Vec<GPUBufferResourceView> {
    self
      .dispatch_info
      .device_ranges
      .create_indirect_count_views()
  }
}
