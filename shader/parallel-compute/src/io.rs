use crate::*;

impl<T: ShaderSizedValueNodeType> DeviceInvocation<Node<T>>
  for Node<ShaderReadOnlyStoragePtr<[T]>>
{
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<T>, Node<bool>) {
    let idx = logic_global_id.x();
    let r = idx.less_than(self.array_length());
    let result = r.select_branched(|| self.index(idx).load(), || zeroed_val());
    (result, r)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    (self.array_length(), val(0), val(0)).into()
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

  fn result_size(&self) -> u32 {
    self.item_count()
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

impl<T> DeviceParallelCompute<Node<T>> for Vec<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    let gpu_buffer = self.materialize_storage_buffer(cx);
    Box::new(StorageBufferReadOnlyDataViewReadIntoShader(gpu_buffer))
  }

  fn result_size(&self) -> u32 {
    self.len() as u32
  }
}
impl<T> DeviceParallelComputeIO<T> for Vec<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> StorageBufferReadOnlyDataView<[T]>
  where
    Self: Sized,
    T: Std430 + ShaderSizedValueNodeType,
  {
    create_gpu_readonly_storage(self.as_slice(), cx.gpu)
  }
}

#[pollster::test]
async fn test_storage_buffer() {
  let input = vec![1_u32; 70];
  let expect = input.clone();

  input.single_run_test(&expect).await
}

pub struct StorageBufferReadOnlyDataViewReadIntoShader<T: Std430>(
  pub StorageBufferReadOnlyDataView<[T]>,
);

impl<T> DeviceInvocationComponent<Node<T>> for StorageBufferReadOnlyDataViewReadIntoShader<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn work_size(&self) -> Option<u32> {
    self.0.item_count().into()
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }
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
impl<T: Std430> ShaderHashProvider for StorageBufferReadOnlyDataViewReadIntoShader<T> {
  shader_hash_type_id! {}
}

pub struct WriteIntoStorageWriter<T: Std430> {
  pub inner: Box<dyn DeviceInvocationComponent<Node<T>>>,
  pub result_write_idx: Box<dyn Fn(Node<u32>) -> Node<u32>>,
  pub output: StorageBufferDataView<[T]>,
}

impl<T: Std430> ShaderHashProvider for WriteIntoStorageWriter<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.inner.hash_pipeline_with_type_info(hasher)
  }
  shader_hash_type_id! {}
}

impl<T> DeviceInvocationComponent<Node<T>> for WriteIntoStorageWriter<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn requested_workgroup_size(&self) -> Option<u32> {
    self.inner.requested_workgroup_size()
  }

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

    invocation_source
      .adhoc_invoke_with_self_size(r)
      .into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.inner.bind_input(builder);
    builder.bind(&self.output);
  }

  fn work_size(&self) -> Option<u32> {
    self.inner.work_size()
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

  write.dispatch_compute(cx);

  write.output.into_readonly_view()
}

pub fn do_write_into_storage_buffer<T: Std430 + ShaderSizedValueNodeType>(
  source: &(impl DeviceParallelComputeIO<T> + ?Sized),
  cx: &mut DeviceParallelComputeCtx,
) -> StorageBufferReadOnlyDataView<[T]> {
  custom_write_into_storage_buffer(source, cx, |x| x)
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
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

  fn result_size(&self) -> u32 {
    self.inner.result_size()
  }
}

impl<T: ShaderSizedValueNodeType + Std430> DeviceParallelComputeIO<T>
  for WriteIntoStorageReadBackToDevice<T>
{
  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> StorageBufferReadOnlyDataView<[T]> {
    do_write_into_storage_buffer(&self.inner, cx)
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DeviceParallelComputeIODebug<T> {
  pub inner: Box<dyn DeviceParallelComputeIO<T>>,
  pub label: &'static str,
}

impl<T: ShaderSizedValueNodeType + Std430 + Debug> DeviceParallelCompute<Node<T>>
  for DeviceParallelComputeIODebug<T>
{
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    let (device_result, host_result) = pollster::block_on(self.inner.read_back_host(cx)).unwrap();

    println!("{}: {:?}", self.label, host_result);

    // todo, log should not has any side effect, but we can not return the inner execute_and_expose
    // because forker expect the execute_and_expose is consumed once
    Box::new(StorageBufferReadOnlyDataViewReadIntoShader(device_result))
  }

  fn result_size(&self) -> u32 {
    self.inner.result_size()
  }
}

/// this impl should not call internal materialization or default implementation, because we have
/// configured the workgroup size
impl<T: ShaderSizedValueNodeType + Std430 + Debug> DeviceParallelComputeIO<T>
  for DeviceParallelComputeIODebug<T>
{
}
