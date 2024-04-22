use crate::*;

impl<T: ShaderSizedValueNodeType> DeviceInvocation<Node<T>>
  for Node<ShaderReadOnlyStoragePtr<[T]>>
{
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<T>, Node<bool>) {
    let idx = logic_global_id.x();
    let r = idx.less_than(self.array_length());
    (r.select(self.index(idx).load(), zeroed_val()), r)
  }
}

impl<T> DeviceParallelCompute<Node<T>> for StorageBufferReadOnlyDataView<[T]>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn execute_and_expose(
    &self,
    _cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    Box::new(StorageBufferReadOnlyDataViewReadIntoShader(self.clone()))
  }

  fn work_size(&self) -> u32 {
    let size: u64 = self.view_byte_size().into();
    let count = size / std::mem::size_of::<T>() as u64;
    count as u32
  }
}
impl<T> DeviceParallelComputeIO<T> for StorageBufferReadOnlyDataView<[T]>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn materialize_storage_buffer(
    &self,
    _: &mut DeviceParallelComputeCtx,
  ) -> StorageBufferReadOnlyDataView<[T]>
  where
    Self: Sized,
    T: Std430 + ShaderSizedValueNodeType,
  {
    self.clone()
  }
}

pub struct StorageBufferReadOnlyDataViewReadIntoShader<T: Std430>(
  pub StorageBufferReadOnlyDataView<[T]>,
);

impl<T> DeviceInvocationComponent<Node<T>> for StorageBufferReadOnlyDataViewReadIntoShader<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    let view = builder.entry_by(|cx| cx.bind_by(&self.0));
    Box::new(view) as Box<dyn DeviceInvocation<Node<T>>>
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.0);
  }
}
impl<T: Std430> ShaderHashProvider for StorageBufferReadOnlyDataViewReadIntoShader<T> {}

pub struct WriteIntoStorageWriter<T: Std430> {
  pub inner: Box<dyn DeviceInvocationComponent<Node<T>>>,
  pub result_write_idx: Box<dyn Fn(Node<u32>) -> Node<u32>>,
  pub output: StorageBufferDataView<[T]>,
}

impl<T: Std430> ShaderHashProvider for WriteIntoStorageWriter<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.inner.hash_pipeline_with_type_info(hasher)
  }
}

impl<T> DeviceInvocationComponent<Node<T>> for WriteIntoStorageWriter<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    let invocation_source = self.inner.build_shader(builder);

    let r = builder.entry_by(|cx| {
      let invocation_id = cx.global_invocation_id();
      let output = cx.bind_by(&self.output);
      let (r, valid) = invocation_source.invocation_logic(invocation_id);

      if_by(valid, || {
        let target_idx = (self.result_write_idx)(invocation_id.x());
        output.index(target_idx).store(r);
      });

      (r, valid)
    });
    Box::new(AdhocInvocationResult(r.0, r.1))
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.inner.bind_input(builder);
    builder.bind(&self.output);
  }
}

pub fn custom_write_into_storage_buffer<T: Std430 + ShaderSizedValueNodeType>(
  source: &(impl DeviceParallelComputeIO<T> + ?Sized),
  cx: &mut DeviceParallelComputeCtx,
  write_target: impl Fn(Node<u32>) -> Node<u32> + 'static,
) -> StorageBufferReadOnlyDataView<[T]> {
  let input_source = source.execute_and_expose(cx);
  let output = create_gpu_read_write_storage::<[T]>(source.result_size() as usize, &cx.gpu);

  let write = WriteIntoStorageWriter {
    inner: input_source,
    result_write_idx: Box::new(write_target),
    output,
  };

  write.dispatch_compute(source.work_size(), cx);

  write.output.into_readonly_view()
}

pub fn do_write_into_storage_buffer<T: Std430 + ShaderSizedValueNodeType>(
  source: &(impl DeviceParallelComputeIO<T> + ?Sized),
  cx: &mut DeviceParallelComputeCtx,
) -> StorageBufferReadOnlyDataView<[T]> {
  custom_write_into_storage_buffer(source, cx, |x| x)
}

pub struct WriteIntoStorageReadBackToDevice<T> {
  pub inner: Box<dyn DeviceParallelComputeIO<T>>,
}

impl<T: ShaderSizedValueNodeType + Std430> DeviceParallelCompute<Node<T>>
  for WriteIntoStorageReadBackToDevice<T>
{
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    let temp_result = self.materialize_storage_buffer(cx);
    Box::new(StorageBufferReadOnlyDataViewReadIntoShader(temp_result))
  }

  fn work_size(&self) -> u32 {
    self.inner.work_size()
  }
}

/// this impl should not call internal materialization or default implementation, because we have
/// configured the workgroup size
impl<T: ShaderSizedValueNodeType + Std430> DeviceParallelComputeIO<T>
  for WriteIntoStorageReadBackToDevice<T>
{
  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> StorageBufferReadOnlyDataView<[T]> {
    do_write_into_storage_buffer(self, cx)
  }
}
