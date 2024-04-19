use crate::*;

impl<T: ShaderNodeType> DeviceInvocation<T> for Node<ShaderReadOnlyStoragePtr<[T]>> {
  fn invocation_logic(&self, cx: &mut ComputeCx) -> (Node<T>, Node<bool>) {
    let idx = cx.global_invocation_id().x();
    let r = idx.less_than(self.array_length());
    (r.select(self.index(idx).load(), zero_shader_value()), r)
  }
}

impl<T> DeviceParallelCompute<T> for StorageBufferReadOnlyDataView<[T]>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn compute_result(
    &self,
    _cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationBuilder<T>> {
    Box::new(StorageBufferReadOnlyDataViewReadIntoShader(self.clone()))
  }

  fn work_size(&self) -> u32 {
    let size: u64 = self.view_byte_size().into();
    let count = size / std::mem::size_of::<T>() as u64;
    count as u32
  }
}

struct StorageBufferReadOnlyDataViewReadIntoShader<T: Std430>(StorageBufferReadOnlyDataView<[T]>);

impl<T> DeviceInvocationBuilder<T> for StorageBufferReadOnlyDataViewReadIntoShader<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<T>> {
    let view = builder.entry_by(|cx| cx.bind_by(&self.0));
    Box::new(view) as Box<dyn DeviceInvocation<T>>
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.0);
  }
}
impl<T: Std430> ShaderHashProvider for StorageBufferReadOnlyDataViewReadIntoShader<T> {}

pub(crate) fn write_into_storage_buffer<T: Std430 + ShaderSizedValueNodeType>(
  source: &impl DeviceParallelCompute<T>,
  cx: &mut DeviceParallelComputeCtx,
) -> StorageBufferDataView<[T]> {
  let group_size = 256;
  let input_source = source.compute_result(cx);

  let output = create_gpu_read_write_storage::<[T]>(source.work_size() as usize, &cx.gpu);

  let pipeline = cx.get_or_create_compute_pipeline(&input_source, |cx| {
    cx.config_work_group_size(group_size);
    let source = input_source.build_shader(cx.0);
    let output = cx.bind_by(&output);

    let (r, valid) = source.invocation_logic(cx);

    if_by(valid, || {
      output.index(cx.global_invocation_id().x()).store(r);
    });
  });

  let encoder = cx.gpu.create_encoder().compute_pass_scoped(|mut pass| {
    let mut bb = BindingBuilder::new_as_compute();
    input_source.bind_input(&mut bb);

    bb.bind(&output)
      .setup_compute_pass(&mut pass, &cx.gpu.device, &pipeline);
    pass.dispatch_workgroups((source.work_size() + group_size - 1) / group_size, 1, 1);
  });

  cx.gpu.submit_encoder(encoder);

  output
}

pub struct WriteStorageReadBack<T> {
  pub inner: Box<dyn DeviceParallelCompute<T>>,
}

impl<T: ShaderSizedValueNodeType + Std430> DeviceParallelCompute<T> for WriteStorageReadBack<T> {
  fn compute_result(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationBuilder<T>> {
    let temp_result = write_into_storage_buffer(&self.inner, cx).into_readonly_view();
    Box::new(StorageBufferReadOnlyDataViewReadIntoShader(temp_result))
  }

  fn work_size(&self) -> u32 {
    self.inner.work_size()
  }
}
