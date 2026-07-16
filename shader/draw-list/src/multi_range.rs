use crate::*;

#[derive(Clone)]
pub struct DeviceMultiRangeDispatchInfo {
  pub sub_list_ranges: StorageBufferReadonlyDataView<[StorageSubListRangeInfo]>,
  // /// sum of all count field of sub_list_ranges
  pub sum_all_count: StorageBufferReadonlyDataView<u32>,
}

impl DeviceMultiRangeDispatchInfo {
  pub fn new(gpu: &GPU, init: &[StorageSubListRangeInfo]) -> Self {
    let device = &gpu.device;
    let sum: u32 = init.iter().map(|v| v.count).sum();
    let sum_all_count = create_gpu_readonly_storage(&sum, gpu, "sum count of multi range");

    let sub_list_ranges = StorageBufferReadonlyDataView::create_by_with_extra_usage(
      device.as_ref(),
      StorageBufferInit::<[StorageSubListRangeInfo]>::from(init),
      BufferUsages::INDIRECT,
      "multi range info",
    );

    Self {
      sub_list_ranges,
      sum_all_count,
    }
  }
  pub fn update(&self, gpu: &GPU, ranges: &[StorageSubListRangeInfo]) {
    let sum: u32 = ranges.iter().map(|v| v.count).sum();
    gpu
      .queue
      .write_buffer(&self.sub_list_ranges.buffer.gpu(), 0, cast_slice(ranges));

    gpu
      .queue
      .write_buffer(&self.sum_all_count.buffer.gpu(), 0, cast_slice(&[sum]));
  }

  pub fn create_indirect_count_views(&self) -> Vec<GPUBufferResourceView> {
    let list_count = self.sub_list_ranges.item_count();
    let mut views = Vec::with_capacity(list_count as usize);
    let buffer = &self.sub_list_ranges;
    assert_eq!(buffer.desc.offset, 0); // we could support this case, but we want to keep it simple
    let elem_stride = std::mem::size_of::<StorageSubListRangeInfo>() as u64;
    for i in 0..list_count {
      let view = buffer.resource.create_view(GPUBufferViewRange {
        offset: elem_stride * i as u64 + 4,
        size: std::num::NonZeroU64::new(4).into(),
      });
      views.push(view);
    }
    views
  }
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

pub struct DeviceMultiRangeDispatchInfoInvocation {
  pub sub_list_ranges: ShaderReadonlyPtrOf<[StorageSubListRangeInfo]>,
  // /// sum of all count field of sub_list_ranges
  pub sum_all_count: ShaderReadonlyPtrOf<u32>,
}

impl DeviceMultiRangeDispatchInfoInvocation {
  pub fn compute_list_index(&self, global_id: Node<u32>) -> (Node<u32>, Node<bool>) {
    let size_all = self.sum_all_count.load();
    let in_bound = global_id.less_than(size_all);

    let sub_list_count = self.sub_list_ranges.array_length();

    // Binary search for the sub-list containing global_id
    // Find the last index i where sub_list_ranges[i].count_prefix_sum <= global_id
    let low = val(0u32).make_local_var();
    let high = sub_list_count.make_local_var();
    let found = val(0u32).make_local_var();

    loop_by(|cx| {
      let lo = low.load();
      let hi = high.load();
      let done = lo.greater_than(hi).or(lo.equals(hi));
      if_by(done, || cx.do_break());

      let mid = (lo + hi) / val(2u32);
      let prefix_sum = self.sub_list_ranges.index(mid).count_prefix_sum().load();

      let p_le_id = prefix_sum
        .less_than(global_id)
        .or(prefix_sum.equals(global_id));
      if_by(p_le_id, || {
        found.store(mid);
        low.store(mid + val(1u32));
      })
      .else_by(|| {
        high.store(mid);
      });
    });

    let list_index = found.load();

    (list_index, in_bound)
  }

  pub fn read_range_info(&self, list_index: Node<u32>) -> ENode<StorageSubListRangeInfo> {
    self.sub_list_ranges.index(list_index).load().expand()
  }
}

impl DeviceMultiRangeDispatchInfo {
  pub fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> DeviceMultiRangeDispatchInfoInvocation {
    DeviceMultiRangeDispatchInfoInvocation {
      sub_list_ranges: builder.bind_by(&self.sub_list_ranges),
      sum_all_count: builder.bind_by(&self.sum_all_count),
    }
  }

  pub fn bind_shader(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.sub_list_ranges);
    builder.bind(&self.sum_all_count);
  }
}
