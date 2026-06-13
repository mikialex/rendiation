use dyn_clone::DynClone;
use rendiation_device_parallel_compute::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod device_culling;
mod list_access;
mod stream_compact;
pub use device_culling::*;

#[derive(Clone)]
pub struct DeviceDrawList {
  pub id_pool: StorageBufferReadonlyDataView<[u32]>,
  pub dispatch_info: MultiRangeDispatchInfo,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, Debug, PartialEq, ShaderStruct)]
pub struct StorageSubListRangeInfo {
  /// pool_read_range_offset
  pub offset: u32,
  /// count
  pub count: u32,
  /// count_prefix_sum(exclusive)
  pub count_prefix_sum: u32,
}

impl StorageSubListRangeInfo {
  pub fn new(offset: u32, count: u32, count_prefix_sum: u32) -> Self {
    StorageSubListRangeInfo {
      offset,
      count,
      count_prefix_sum,
      ..Zeroable::zeroed()
    }
  }
}

#[derive(Clone)]
pub struct MultiRangeDispatchInfo {
  pub sub_list_ranges: StorageBufferReadonlyDataView<[StorageSubListRangeInfo]>,
  // /// sum of all count field of sub_list_ranges, used for computing indirect draw parameter
  pub sum_all_count: StorageBufferReadonlyDataView<u32>,
  pub host_capacity_ranges: Vec<CapacityRange>,
  pub sum_all_count_host: u32,
}

#[derive(Clone)]
pub struct CapacityRange {
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

    ranges.push(StorageSubListRangeInfo {
      offset: info.offset,
      count: length,
      count_prefix_sum: prefix_sum,
      ..Zeroable::zeroed()
    });
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
      Some(existing) => existing.dispatch_info.sum_all_count_host != total_capacity,
      None => true,
    };

    // the real count and offsets are override by compute shader write
    let length = vec![0_u32; sub_list_infos.len()];
    let ranges_init = prepare_gpu_sub_list_ranges(sub_list_infos, &length);
    if needs_create {
      let sub_list_ranges = StorageBufferReadonlyDataView::create_by_with_extra_usage(
        gpu.device.as_ref(),
        Some("device draw list sub_list_ranges"),
        StorageBufferInit::<[StorageSubListRangeInfo]>::from(ranges_init.as_slice()),
        BufferUsages::INDIRECT,
      );

      let pool_data = vec![0u32; total_capacity as usize];
      let scene_model_id_pool = create_gpu_readonly_storage(pool_data.as_slice(), gpu);
      let sum_all_count = create_gpu_readonly_storage(&0u32, gpu);

      *cached = Some(DeviceDrawList {
        id_pool: scene_model_id_pool,
        dispatch_info: MultiRangeDispatchInfo {
          sub_list_ranges,
          sum_all_count,
          host_capacity_ranges: sub_list_infos.to_vec(),
          sum_all_count_host: total_capacity,
        },
      });
    } else {
      // Reset the cached output's sub_list_ranges counts to zero; the GPU
      // compute pass will overwrite them with real survival counts.
      let target = cached.as_ref().unwrap();
      gpu.queue.write_buffer(
        &target.dispatch_info.sub_list_ranges.buffer.gpu(),
        0,
        cast_slice(ranges_init.as_slice()),
      );
    }

    cached.as_ref().unwrap()
  }

  pub fn create_indirect_count_views(&self) -> Vec<GPUBufferResourceView> {
    let mut views = Vec::with_capacity(self.dispatch_info.host_capacity_ranges.len());
    let buffer = &self.dispatch_info.sub_list_ranges;
    assert_eq!(buffer.desc.offset, 0); // we could support this case, but we want to keep it simple
    let elem_stride = std::mem::size_of::<StorageSubListRangeInfo>() as u64;
    for i in 0..self.dispatch_info.host_capacity_ranges.len() {
      let view = buffer.resource.create_view(GPUBufferViewRange {
        offset: elem_stride * i as u64 + 4,
        size: std::num::NonZeroU64::new(4).into(),
      });
      views.push(view);
    }
    views
  }
}
