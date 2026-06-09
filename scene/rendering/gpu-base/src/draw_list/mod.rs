use crate::*;
mod list_access;
mod predicate;
pub use predicate::*;
mod scatter;
pub use scatter::*;
#[cfg(test)]
mod tests;

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
  /// this id is only used for implementation selecting. itself may be not included in list.
  pub impl_select_id: EntityHandle<SceneModelEntity>,
  /// this capacity is to allocate the necessary space when do filtering, as we
  /// can not read back real length from gpu in frame.
  pub capacity: u32,
  pub offset: u32,
}

impl DeviceDrawList {
  /// Creates (or reuses from cache) a DeviceDrawList with pre-allocated output buffers sized
  /// according to per-sub-list capacities. The sub_list_ranges are initialized with zero counts;
  /// the GPU fills in actual survival counts during culling.
  pub fn create_or_update_compact_write_target<'a>(
    &self,
    gpu: &GPU,
    cached: &'a mut Option<Self>,
  ) -> &'a Self {
    let total_capacity: u32 = self
      .dispatch_info
      .sub_list_infos
      .iter()
      .map(|info| info.capacity)
      .sum();

    // Reuse cached target if the total capacity matches.
    let needs_create = match cached.as_ref() {
      Some(existing) => existing.dispatch_info.sum_all_count_host != total_capacity,
      None => true,
    };

    if needs_create {
      let sub_count = self.dispatch_info.sub_list_infos.len();
      let mut offset = 0u32;
      let mut ranges_init = Vec::with_capacity(sub_count);
      for info in self.dispatch_info.sub_list_infos.iter() {
        ranges_init.push(Vec4::new(offset, 0, 0, 0));
        offset += info.capacity;
      }

      let sub_list_ranges = create_gpu_readonly_storage(ranges_init.as_slice(), gpu);
      let pool_data = vec![0u32; total_capacity as usize];
      let scene_model_id_pool = create_gpu_readonly_storage(pool_data.as_slice(), gpu);
      let sum_all_count = create_gpu_readonly_storage(&0u32, gpu);

      *cached = Some(DeviceDrawList {
        scene_model_id_pool,
        dispatch_info: MultiRangeDispatchInfo {
          sub_list_ranges,
          sum_all_count,
          sub_list_infos: self.dispatch_info.sub_list_infos.clone(),
          sum_all_count_host: total_capacity,
        },
      });
    }

    cached.as_ref().unwrap()
  }

  pub fn use_culled_list_and_do_culling(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    culler: Box<dyn AbstractCullerProvider>,
  ) -> Self {
    let gpu = cx.gpu.clone();

    let (cx, target_state) = cx.use_plain_state_default::<Option<DeviceDrawList>>();
    let target = self.create_or_update_compact_write_target(&gpu, target_state);

    let output_pool = target.scene_model_id_pool.clone().into_rw_view();
    let output_ranges = target.dispatch_info.sub_list_ranges.clone().into_rw_view();
    let total_count_out = target.dispatch_info.sum_all_count.clone().into_rw_view();

    let predicate = ListOfListsCullingPredicate {
      draw_list: self.clone(),
      culler: culler.clone(),
    };
    let positions =
      predicate.segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>(1024, 1024, cx);
    let scatter = SegmentedListScatter {
      positions: positions.buffer.clone(),
      sub_list_ranges: self.dispatch_info.sub_list_ranges.clone(),
      draw_list: self.clone(),
      output_pool,
      output_ranges,
      total_count_out,
    };
    scatter.dispatch_compute(cx);

    DeviceDrawList {
      scene_model_id_pool: scatter.output_pool.into_readonly_view(),
      dispatch_info: MultiRangeDispatchInfo {
        sub_list_ranges: scatter.output_ranges.into_readonly_view(),
        sum_all_count: scatter.total_count_out.into_readonly_view(),
        sub_list_infos: self.dispatch_info.sub_list_infos.clone(),
        sum_all_count_host: target.dispatch_info.sum_all_count_host,
      },
    }
  }
}
