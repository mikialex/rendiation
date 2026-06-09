use crate::*;
mod list_access;

#[derive(Clone)]
pub struct DeviceDrawList {
  pub scene_model_id_pool: StorageBufferReadonlyDataView<[u32]>,
  pub dispatch_info: MultiRangeDispatchInfo,
}

#[derive(Clone)]
pub struct MultiRangeDispatchInfo {
  /// (offset, count, count_prefix_sum)
  pub sub_list_ranges: StorageBufferReadonlyDataView<[Vec3<u32>]>,
  // /// sum of all count field of sub_list_ranges, used for computing indirect draw parameter
  pub sum_all_count: StorageBufferReadonlyDataView<u32>,
  pub sub_list_infos: Vec<SubListHostInfo>,
  pub sum_all_count_host: u32,
}

// fn regroup(info: &MultiRangeDispatchInfo) -> MultiRangeDispatchInfo {
//   todo!()
// }

#[derive(Clone)]
pub struct SubListHostInfo {
  /// this id is only used for implementation selecting. itself may be not included in list.
  pub impl_select_id: EntityHandle<SceneModelEntity>,
  /// this capacity is to allocate the necessary space when do filtering, as we
  /// can not read back real length from gpu in frame.
  pub capacity: u32,
  pub offset: u32,
}

impl DeviceDrawList {
  pub fn create_or_update_compact_write_target<'a>(
    &self,
    gpu: &GPU,
    cached: &'a mut Option<Self>,
  ) -> &'a Self {
    // let mut sum = 0;
    // let mut sub_list_offsets_count_prefix_sum = Vec::with_capacity(self.sub_list_infos.len());
    // for list in self.sub_list_infos.iter() {
    //   // the count and count_prefix_sum are zero, and should be write by device later.
    //   sub_list_offsets_count_prefix_sum.push(Vec3::new(sum, 0, 0));
    //   sum += list.capacity;
    // }
    // let sub_list_ranges =
    //   create_gpu_readonly_storage(sub_list_offsets_count_prefix_sum.as_slice(), &device);
    // let scene_model_id_pool = todo!();

    // DeviceDrawList {
    //   scene_model_id_pool,
    //   sub_list_ranges,
    //   sum_all_count: todo!(),
    //   sub_list_infos: self.sub_list_infos.clone(),
    //   sum_all_count_host: sum,
    // }
    todo!()
  }

  pub fn use_culled_list_and_do_culling(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    culler: Box<dyn AbstractCullerProvider>,
  ) -> Self {
    let gpu = cx.gpu.clone();
    let (cx, target) = cx.use_plain_state_default::<Option<DeviceDrawList>>();
    let target = self.create_or_update_compact_write_target(&gpu, target);

    // stream_compaction_list_of_lists

    todo!();

    target.clone()
  }
}
