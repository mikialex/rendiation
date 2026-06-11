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
  pub scene_model_id_pool: StorageBufferReadonlyDataView<[u32]>,
  pub dispatch_info: MultiRangeDispatchInfo,
}

#[derive(Clone)]
pub struct MultiRangeDispatchInfo {
  /// (offset, count, count_prefix_sum, _padding) — Vec4 for 16B storage alignment
  pub sub_list_ranges: StorageBufferReadonlyDataView<[Vec4<u32>]>,
  // /// sum of all count field of sub_list_ranges, used for computing indirect draw parameter
  pub sum_all_count: StorageBufferReadonlyDataView<u32>,
  pub sub_list_infos: Vec<SubListHostInfo>,
  pub sum_all_count_host: u32,
}

#[derive(Clone)]
pub struct SubListHostInfo {
  /// this capacity is to allocate the necessary space when do filtering, as we
  /// can not read back real length from gpu in frame.
  pub capacity: u32,
  pub offset: u32,
}

pub fn compute_gpu_sub_list_ranges(sub_list_infos: &[SubListHostInfo]) -> Vec<Vec4<u32>> {
  let sub_count = sub_list_infos.len();
  let mut offset = 0u32;
  let mut ranges = Vec::with_capacity(sub_count);
  for info in sub_list_infos.iter() {
    // prefix sum is equal to offset in this case
    ranges.push(Vec4::new(offset, info.capacity, offset, 0));
    offset += info.capacity;
  }
  ranges
}

impl DeviceDrawList {
  /// Creates (or reuses from cache) a DeviceDrawList with pre-allocated output buffers sized
  /// according to per-sub-list capacities. The sub_list_ranges are initialized with zero counts;
  /// the GPU fills in actual survival counts during culling.
  pub fn create_or_update_compact_write_target<'a>(
    &self,
    gpu: &GPU,
    cached: &'a mut Option<Self>,
    sub_list_infos: &[SubListHostInfo],
  ) -> &'a Self {
    let total_capacity: u32 = sub_list_infos.iter().map(|info| info.capacity).sum();

    // Reuse cached target if the total capacity matches.
    let needs_create = match cached.as_ref() {
      Some(existing) => existing.dispatch_info.sum_all_count_host != total_capacity,
      None => true,
    };

    // the real count and offsets may be override by compute shader write
    let ranges_init = compute_gpu_sub_list_ranges(sub_list_infos);
    if needs_create {
      let sub_list_ranges = StorageBufferReadonlyDataView::create_by_with_extra_usage(
        gpu.device.as_ref(),
        Some("device draw list sub_list_ranges"),
        StorageBufferInit::<[Vec4<u32>]>::from(ranges_init.as_slice()),
        BufferUsages::INDIRECT,
      );

      let pool_data = vec![0u32; total_capacity as usize];
      let scene_model_id_pool = create_gpu_readonly_storage(pool_data.as_slice(), gpu);
      let sum_all_count = create_gpu_readonly_storage(&0u32, gpu);

      *cached = Some(DeviceDrawList {
        scene_model_id_pool,
        dispatch_info: MultiRangeDispatchInfo {
          sub_list_ranges,
          sum_all_count,
          sub_list_infos: sub_list_infos.to_vec(),
          sum_all_count_host: total_capacity,
        },
      });
    } else {
      // make sure the offset field is always updated.
      gpu.queue.write_buffer(
        &self.dispatch_info.sub_list_ranges.buffer.gpu(),
        0,
        cast_slice(ranges_init.as_slice()),
      );
    }

    cached.as_ref().unwrap()
  }

  pub fn create_indirect_count_views(&self) -> Vec<GPUBufferResourceView> {
    let mut views = Vec::with_capacity(self.dispatch_info.sub_list_infos.len());
    let buffer = &self.dispatch_info.sub_list_ranges;
    assert_eq!(buffer.desc.offset, 0); // we could support this case, but we want to keep it simple
    for i in 0..self.dispatch_info.sub_list_infos.len() {
      let view = buffer.resource.create_view(GPUBufferViewRange {
        offset: 4 * 4 * i as u64 + 4,
        size: std::num::NonZeroU64::new(4).into(),
      });
      views.push(view);
    }
    views
  }
}
