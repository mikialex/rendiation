use crate::*;

pub trait ComputeComponent<T>: ShaderHashProvider + DynClone {
  fn clone_boxed(&self) -> Box<dyn ComputeComponent<T>>;

  /// return None if the real work size is not known at host side
  fn work_size(&self) -> Option<u32>;

  /// If the materialized output size is different from `work_size`
  /// (for example in reduction operation), a custom implementation is required to override the method
  fn result_size(&self) -> u32;

  fn requested_workgroup_size(&self) -> Option<u32>;

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<T>>;

  fn bind_input(&self, builder: &mut BindingBuilder);

  fn dispatch_compute(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Option<StorageBufferReadonlyDataView<Vec4<u32>>> {
    if !cx.force_indirect_dispatch && self.work_size().is_some() {
      let workgroup_size = self.requested_workgroup_size().unwrap_or(256);
      self.prepare_main_pass(cx);
      let work_size = self.work_size().unwrap();
      cx.record_pass(|pass, _| {
        pass.dispatch_workgroups(compute_dispatch_size(work_size, workgroup_size), 1, 1);
      });
      None
    } else {
      let (indirect_dispatch_size, indirect_work_size) = self.compute_work_size(cx);
      self.prepare_main_pass(cx);
      cx.record_pass(|pass, _| {
        pass.dispatch_workgroups_indirect_by_buffer_resource_view(&indirect_dispatch_size);
      });
      Some(indirect_work_size.into_readonly_view())
    }
  }

  fn prepare_main_pass(&self, cx: &mut DeviceParallelComputeCtx) {
    let workgroup_size = self.requested_workgroup_size().unwrap_or(256);
    let main_pipeline = cx.get_or_create_compute_pipeline(self, |cx| {
      cx.config_work_group_size(workgroup_size);
      let invocation_source = self.build_shader(cx);

      let invocation_id = cx.global_invocation_id();
      let _ = invocation_source.invocation_logic(invocation_id);
    });
    cx.record_pass(|pass, device| {
      let mut bb = BindingBuilder::default();
      self.bind_input(&mut bb);
      bb.setup_compute_pass(pass, device, &main_pipeline);
    });
  }

  fn compute_work_size(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> (
    StorageBufferDataView<DispatchIndirectArgsStorage>,
    StorageBufferDataView<Vec4<u32>>,
  ) {
    struct SizeWriter<'a, T: ?Sized>(&'a T);
    impl<T: ShaderHashProvider + ?Sized> ShaderHashProvider for SizeWriter<'_, T> {
      fn hash_type_info(&self, hasher: &mut PipelineHasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(hasher);
        self.0.hash_type_info(hasher)
      }

      fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
        self.0.hash_pipeline(hasher)
      }
    }

    let size_output = cx.gpu.device.make_indirect_dispatch_size_buffer();
    let work_size_output = StorageBufferReadonlyDataView::create_by_with_extra_usage(
      &cx.gpu.device,
      Some("work_size_output"),
      StorageBufferInit::WithInit(&Vec4::<u32>::zero()),
      BufferUsages::INDIRECT,
    )
    .into_rw_view();

    // requested_workgroup_size should always be respected
    let workgroup_size = self.requested_workgroup_size().unwrap_or(256);
    let workgroup_size_buffer =
      create_gpu_readonly_storage(&workgroup_size, &cx.gpu.device).into_rw_view();

    let pipeline = cx.get_or_create_compute_pipeline(&SizeWriter(self), |cx| {
      cx.config_work_group_size(workgroup_size);

      let size_output = cx.bind_by(&size_output);
      let work_size_output = cx.bind_by(&work_size_output);
      let workgroup_size = cx.bind_by(&workgroup_size_buffer);

      let size = self.build_shader(cx).invocation_size();
      let size: Node<Vec4<u32>> = (size, val(0)).into();

      work_size_output.store(size);

      let size = ENode::<DispatchIndirectArgsStorage> {
        x: device_compute_dispatch_size(size.x(), workgroup_size.load()),
        y: size.y().max(1),
        z: size.z().max(1),
      }
      .construct();

      size_output.store(size);
    });

    cx.record_pass(|pass, device| {
      BindingBuilder::default()
        .with_bind(&size_output)
        .with_bind(&work_size_output)
        .with_bind(&workgroup_size_buffer)
        .with_fn(|bb| self.bind_input(bb))
        .setup_compute_pass(pass, device, &pipeline);
      pass.dispatch_workgroups(1, 1, 1);
    });

    (size_output, work_size_output)
  }
}

pub trait ComputeComponentIO<T>: ComputeComponent<Node<T>> {
  /// The user must not mutate the materialized result returned from this function.
  ///
  /// If the implementation has already materialized the storage buffer internally, a custom implementation
  /// should override this method to expose the result directly and avoid re-materialization cost.
  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    T: Std430 + ShaderSizedValueNodeType,
    Self: Sized,
  {
    let init = ZeroedArrayByArrayLength(self.result_size() as usize);
    let output = create_gpu_read_write_storage::<[T]>(init, &cx.gpu);
    self.materialize_storage_buffer_into(output, cx)
  }

  fn materialize_storage_buffer_into(
    &self,
    target: StorageBufferDataView<[T]>,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    do_write_into_storage_buffer(self, cx, target)
  }
}

impl<T> Clone for Box<dyn ComputeComponent<T>> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

impl<T> Clone for Box<dyn ComputeComponentIO<T>> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

impl<T: 'static> ShaderHashProvider for Box<dyn ComputeComponent<T>> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher)
  }

  shader_hash_type_id! {}
}

impl<T: 'static> ComputeComponent<T> for Box<dyn ComputeComponent<T>> {
  fn work_size(&self) -> Option<u32> {
    (**self).work_size()
  }

  fn result_size(&self) -> u32 {
    (**self).result_size()
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    (**self).requested_workgroup_size()
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<T>> {
    (**self).build_shader(builder)
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    (**self).bind_input(builder)
  }

  fn clone_boxed(&self) -> Box<dyn ComputeComponent<T>> {
    (**self).clone_boxed()
  }
}

impl<T: 'static> ShaderHashProvider for Box<dyn ComputeComponentIO<T>> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher)
  }

  shader_hash_type_id! {}
}

impl<T: 'static> ComputeComponent<Node<T>> for Box<dyn ComputeComponentIO<T>> {
  fn work_size(&self) -> Option<u32> {
    (**self).work_size()
  }

  fn result_size(&self) -> u32 {
    (**self).result_size()
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    (**self).requested_workgroup_size()
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    (**self).build_shader(builder)
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    (**self).bind_input(builder)
  }

  fn clone_boxed(&self) -> Box<dyn ComputeComponent<Node<T>>> {
    (**self).clone_boxed()
  }
}

impl<T: 'static> ComputeComponentIO<T> for Box<dyn ComputeComponentIO<T>> {
  fn materialize_storage_buffer_into(
    &self,
    target: StorageBufferDataView<[T]>,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    T: Std430 + ShaderSizedValueNodeType,
    Self: Sized,
  {
    (**self).materialize_storage_buffer_into(target, cx)
  }
}
