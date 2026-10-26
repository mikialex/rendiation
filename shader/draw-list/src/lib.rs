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

/// A list of sub lists of ids, the ids of each sub list are stored in a region of the id pool.
///
/// The layout invariant: for each sub list, the offset in device side range info must equal
/// to the offset in [MultiRangeDispatchInfo::host_capacity_ranges], so the pool region of each
/// sub list is always known at host side.
#[derive(Clone)]
pub struct DeviceDrawList {
  pub id_pool: StorageBufferReadonlyDataView<[u32]>,
  pub dispatch_info: MultiRangeDispatchInfo,
}

#[derive(Clone)]
pub struct MultiRangeDispatchInfo {
  pub device_ranges: DeviceMultiRangeDispatchInfo,
  /// the pool region of each sub list, see [DeviceDrawList] for the layout invariant.
  pub host_capacity_ranges: Vec<CapacityRange>,
  /// The upper bound of the device side sum_all_count, it's used to size the buffers indexed by
  /// the logical element index(the index in the concatenation of all sub lists' real content).
  ///
  /// It must not exceed the sum of all sub lists' capacity, but it can be smaller (for example
  /// the real element count if it's known at host side).
  pub total_capacity: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapacityRange {
  /// must not equal to zero
  pub capacity: u32,
  pub offset: u32,
}

/// Place the sub lists one after another by their capacity, return the new ranges and the
/// total size of the placed pool.
///
/// The input offsets are ignored, if the input capacity is aligned, the output offset is also
/// aligned.
pub fn compact_capacity_ranges(ranges: &[CapacityRange]) -> (Vec<CapacityRange>, u32) {
  let mut offset = 0;
  let compacted = ranges
    .iter()
    .map(|range| {
      let r = CapacityRange {
        capacity: range.capacity,
        offset,
      };
      offset += range.capacity;
      r
    })
    .collect();
  (compacted, offset)
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
  /// Creates (or reuses from cache) a DeviceDrawList with pre-allocated output buffers placed by
  /// [compact_capacity_ranges]. The sub_list_ranges are initialized with zero counts; the GPU
  /// fills in actual survival counts during culling.
  pub(crate) fn create_or_update_compact_culling_write_target<'a>(
    &self,
    gpu: &GPU,
    cached: &'a mut Option<Self>,
  ) -> &'a Self {
    // we do not do any storage buffer binding alignment here because the id pool is
    // not accessed by any sub buffer view
    let (sub_list_infos, pool_size) =
      compact_capacity_ranges(&self.dispatch_info.host_capacity_ranges);

    let needs_create = match cached.as_ref() {
      Some(existing) => {
        existing.id_pool.item_count() != pool_size
          || existing.dispatch_info.host_capacity_ranges.len() != sub_list_infos.len()
      }
      None => true,
    };

    // the real count and prefix sum are override by compute shader write
    let length = vec![0_u32; sub_list_infos.len()];
    let ranges_init = prepare_gpu_sub_list_ranges(&sub_list_infos, &length);
    if needs_create {
      let device_ranges = DeviceMultiRangeDispatchInfo::new(gpu, ranges_init.as_slice());

      let pool_data = vec![0u32; pool_size as usize];
      let id_pool = create_gpu_readonly_storage(
        pool_data.as_slice(),
        gpu,
        "device draw list scene_model_id_pool",
      );

      *cached = Some(DeviceDrawList {
        id_pool,
        dispatch_info: MultiRangeDispatchInfo {
          device_ranges,
          host_capacity_ranges: sub_list_infos,
          total_capacity: self.dispatch_info.total_capacity,
        },
      });
    } else {
      let target = cached.as_mut().unwrap();
      target
        .dispatch_info
        .device_ranges
        .update(gpu, ranges_init.as_slice());
      target.dispatch_info.host_capacity_ranges = sub_list_infos;
      target.dispatch_info.total_capacity = self.dispatch_info.total_capacity;
    }

    cached.as_ref().unwrap()
  }
}
