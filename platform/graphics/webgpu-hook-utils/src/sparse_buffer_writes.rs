use crate::*;

#[derive(Default)]
pub struct SparseBufferWritesSource {
  pub data_to_write: Vec<u8>,
  /// note, the offset size pair must not overlapping with each other
  ///
  /// addressed by u32 unit, not byte!
  ///
  /// packed:(copy src offset, copy src size, copy target offset)
  ///
  /// todo, we should try vec4 vectorized load if has better performance
  pub offset_size: Vec<u32>,
}

/// enable this to debug <potential forget to resize target> bug
const ENABLE_TARGET_SIZE_CHECK: bool = true;
// todo add this check
// enable this to debug <update has overlap> bug
// const ENABLE_NO_OVERLAP_CHECK: bool = true;

impl SparseBufferWritesSource {
  pub fn with_capacity(data_capacity: usize, offset_size_capacity: usize) -> Self {
    Self {
      data_to_write: Vec::with_capacity(data_capacity),
      offset_size: Vec::with_capacity(offset_size_capacity),
    }
  }

  pub fn merge(&mut self, other: SparseBufferWritesSource) {
    let base_offset = self.data_to_write.len() as u32 / 4;

    self.data_to_write.extend_from_slice(&other.data_to_write);
    other.offset_size.into_iter().array_chunks::<3>().for_each(
      |[src_offset, write_size, target_offset]| {
        self.offset_size.push(src_offset + base_offset);
        self.offset_size.push(write_size);
        self.offset_size.push(target_offset);
      },
    );
  }

  // todo, split large write into average size for better load balance?
  pub fn collect_write(&mut self, data_to_write: &[u8], write_offset_in_bytes: u64) {
    assert_eq!(data_to_write.len() % 4, 0);
    assert_eq!(write_offset_in_bytes % 4, 0);

    let src_offset = self.data_to_write.len();
    self.data_to_write.extend_from_slice(data_to_write);

    self.offset_size.push(src_offset as u32 / 4);
    let write_size = data_to_write.len() as u32 / 4;
    self.offset_size.push(write_size);

    let write_offset = write_offset_in_bytes as u32 / 4;
    self.offset_size.push(write_offset);
  }

  pub fn is_empty(&self) -> bool {
    self.offset_size.is_empty()
  }

  pub fn write(
    &self,
    device: &GPUDevice,
    pass: &mut GPUComputePass,
    target_buffer: GPUBufferResourceView,
  ) {
    if self.is_empty() {
      return;
    }

    if ENABLE_TARGET_SIZE_CHECK {
      let mut max_write_size = 0;
      for [_, write_size, target_offset] in self.offset_size.array_chunks::<3>() {
        max_write_size = max_write_size.max(write_size + target_offset);
      }
      assert!(max_write_size <= u64::from(target_buffer.view_byte_size()) as u32 / 4);
    }

    assert_eq!(self.offset_size.len() % 3, 0);

    let data_to_write = cast_slice(&self.data_to_write); // todo, this may panic because unnecessary alignment check
    let data_to_write = create_gpu_readonly_storage::<[u32]>(data_to_write, device);
    let offset_size = create_gpu_readonly_storage::<[u32]>(&self.offset_size, device);

    let target_buffer = StorageBufferDataView::<[u32]>::try_from_raw(target_buffer).unwrap();
    let workgroup_width = 1024; // todo, go wider if device limits support(1024 is min requirement in spec)?

    let hasher = shader_hasher_from_marker_ty!(SparseBufferWrite);
    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder.config_work_group_size(workgroup_width);
      let data_to_write = builder.bind_by(&data_to_write);
      let offset_size = builder.bind_by(&offset_size);
      let target = builder.bind_by(&target_buffer);

      let id = builder.global_invocation_id().x();

      if_by(id.less_than(offset_size.array_length() / val(3)), || {
        let copy_src_offset = offset_size.index(id * val(3)).load();
        let copy_src_size = offset_size.index(id * val(3) + val(1)).load();
        let copy_target_offset = offset_size.index(id * val(3) + val(2)).load();

        let copied_size = val(0_u32).make_local_var();

        loop_by(|cx| {
          let current_copied_size = copied_size.load();
          if_by(current_copied_size.equals(copy_src_size), || cx.do_break());

          let data = data_to_write
            .index(copy_src_offset + current_copied_size)
            .load();
          target
            .index(copy_target_offset + current_copied_size)
            .store(data);

          copied_size.store(current_copied_size + val(1));
        });
      });

      builder
    });

    BindingBuilder::default()
      .with_bind(&data_to_write)
      .with_bind(&offset_size)
      .with_bind(&target_buffer)
      .setup_compute_pass(pass, device, &pipeline);

    let copy_count = self.offset_size.len() as u32 / 3;
    let wg_count = copy_count.div_ceil(workgroup_width);
    pass.dispatch_workgroups(wg_count, 1, 1);
  }
}
