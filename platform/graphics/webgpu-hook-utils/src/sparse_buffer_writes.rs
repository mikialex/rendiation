use crate::*;

#[derive(Default)]
pub struct SparseBufferWritesSource {
  pub data_to_write: Vec<u8>,
  /// note, the offset size pair must not overlapping with each other
  /// addressed by u32 unit, not byte!
  /// (copy src offset, copy src size, copy target offset)
  /// todo, we should try vec4 vectorized load if has better performance
  pub offset_size: Vec<u32>,
}

impl SparseBufferWritesSource {
  // todo, split large write into average size for better load balance?
  pub fn collect_write(&mut self, data_to_write: &[u8], write_offset_in_bytes: u64) {
    assert_eq!(data_to_write.len() % 4, 0);
    assert_eq!(write_offset_in_bytes % 4, 0);

    let src_offset = self.data_to_write.len();
    let copy_size = data_to_write.len() / 4;
    self.data_to_write.extend_from_slice(data_to_write);
    self.offset_size.push(src_offset as u32);
    self.offset_size.push(copy_size as u32);
    self.offset_size.push(write_offset_in_bytes as u32);
  }
}

pub struct SparseBufferWrites {
  /// the target buffer must has storage usage.
  pub target_buffer: GPUBufferResourceView,
  pub source: SparseBufferWritesSource,
}

impl SparseBufferWrites {
  pub fn is_empty(&self) -> bool {
    self.source.offset_size.is_empty()
  }

  pub fn write(self, device: &GPUDevice, pass: &mut GPUComputePass) {
    if self.is_empty() {
      return;
    }

    let source = self.source;

    assert_eq!(source.offset_size.len() % 3, 0);
    let data_to_write = cast_slice(&source.data_to_write); // todo, this may panic because unnecessary alignment check
    let data_to_write = create_gpu_readonly_storage::<[u32]>(data_to_write, device);
    let offset_size = create_gpu_readonly_storage::<[u32]>(&source.offset_size, device);

    let target_buffer = StorageBufferDataView::<[u32]>::try_from_raw(self.target_buffer).unwrap();
    let workgroup_width = 1024; // todo, go wider if device limits support(1024 is spec safe)?

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

    let copy_count = source.offset_size.len() as u32 / 3;
    let wg_count = copy_count.div_ceil(workgroup_width);
    pass.dispatch_workgroups(wg_count, 1, 1);
  }
}
