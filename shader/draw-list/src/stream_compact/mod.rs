mod predicate;
use predicate::*;
mod scatter;
use scatter::*;
#[cfg(test)]
mod tests;

use crate::*;

impl DeviceDrawList {
  pub fn use_culled_list_and_do_culling(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    culler: Box<dyn AbstractCullerProvider>,
  ) -> Self {
    let gpu = cx.gpu.clone();

    let (cx, target_state) = cx.use_plain_state_default::<Option<DeviceDrawList>>();
    let target = self.create_or_update_compact_culling_write_target(
      &gpu,
      target_state,
      &self.dispatch_info.host_capacity_ranges,
    );

    let output_pool = target.id_pool.clone().into_rw_view();
    let device_ranges = &target.dispatch_info.device_ranges;
    let output_ranges = device_ranges.sub_list_ranges.clone().into_rw_view();
    let total_count_out = device_ranges.sum_all_count.clone().into_rw_view();

    let max_width = cx
      .gpu
      .info()
      .supported_limits
      .max_compute_invocations_per_workgroup;

    let predicate = ListOfListsCullingPredicate {
      draw_list: self.clone(),
      culler: culler.clone(),
    };
    let positions = predicate
      .use_segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>(max_width, max_width, cx);
    let scatter = SegmentedListScatter {
      positions: positions.buffer.clone(),
      sub_list_ranges: self.dispatch_info.device_ranges.sub_list_ranges.clone(),
      draw_list: self.clone(),
      output_pool,
      output_ranges,
      total_count_out,
    };
    scatter.use_dispatch_compute(cx);

    DeviceDrawList {
      id_pool: scatter.output_pool.into_readonly_view(),
      dispatch_info: MultiRangeDispatchInfo {
        device_ranges: DeviceMultiRangeDispatchInfo {
          sub_list_ranges: scatter.output_ranges.into_readonly_view(),
          sum_all_count: scatter.total_count_out.into_readonly_view(),
        },
        host_capacity_ranges: self.dispatch_info.host_capacity_ranges.clone(),
        total_capacity: target.dispatch_info.total_capacity,
      },
    }
  }
}
