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

impl<T: ShaderSizedValueNodeType> DeviceInvocation<Node<T>>
  for (Node<ShaderReadOnlyStoragePtr<[T]>>, Node<Vec4<u32>>)
{
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<T>, Node<bool>) {
    let idx = logic_global_id.x();
    let r = idx.less_than(self.1.x());
    let result = r.select_branched(|| self.0.index(idx).load(), || zeroed_val());
    (result, r)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.1.xyz()
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
    Box::new(DeviceMaterializeResult {
      buffer: self.clone(),
      size: None,
    })
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
  ) -> DeviceMaterializeResult<T>
  where
    Self: Sized,
    T: Std430 + ShaderSizedValueNodeType,
  {
    DeviceMaterializeResult::full_buffer(self.clone())
  }

  fn materialize_storage_buffer_into(
    &self,
    target: StorageBufferDataView<[T]>,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    cx.encoder.copy_buffer_to_buffer(
      self.raw_gpu().buffer.gpu(),
      0,
      target.raw_gpu().buffer.gpu(),
      0,
      self.raw_gpu().view_byte_size().into(),
    );
    DeviceMaterializeResult::full_buffer(target.into_readonly_view())
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
    self.materialize_storage_buffer(cx).into_boxed()
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
  ) -> DeviceMaterializeResult<T>
  where
    Self: Sized,
    T: Std430 + ShaderSizedValueNodeType,
  {
    DeviceMaterializeResult::full_buffer(create_gpu_readonly_storage(self.as_slice(), &cx.gpu))
  }
}

#[pollster::test]
async fn test_storage_buffer() {
  let input = vec![1_u32; 70];
  let expect = input.clone();

  input.run_test(&expect).await
}

#[derive(Clone)]
pub struct DeviceMaterializeResult<T: Std430> {
  pub buffer: StorageBufferReadOnlyDataView<[T]>,
  pub size: Option<StorageBufferReadOnlyDataView<Vec4<u32>>>,
}

impl<T: Std430> DeviceMaterializeResult<T> {
  pub fn full_buffer(buffer: StorageBufferReadOnlyDataView<[T]>) -> Self {
    Self { buffer, size: None }
  }
}

impl<T> DeviceInvocationComponent<Node<T>> for DeviceMaterializeResult<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn work_size(&self) -> Option<u32> {
    if self.size.is_some() {
      None
    } else {
      self.buffer.item_count().into()
    }
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    let view = builder.bind_by(&self.buffer);
    if let Some(size) = &self.size {
      let size = builder.bind_by(size).load();
      Box::new((view, size)) as Box<dyn DeviceInvocation<Node<T>>>
    } else {
      Box::new(view) as Box<dyn DeviceInvocation<Node<T>>>
    }
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.buffer);
    if let Some(size) = &self.size {
      builder.bind(size);
    }
  }
}
impl<T: Std430> ShaderHashProvider for DeviceMaterializeResult<T> {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.size.is_some().hash(hasher)
  }
}
impl<T: Std430 + ShaderSizedValueNodeType> DeviceParallelCompute<Node<T>>
  for DeviceMaterializeResult<T>
{
  fn execute_and_expose(
    &self,
    _: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    Box::new(self.clone())
  }

  fn result_size(&self) -> u32 {
    self.buffer.item_count()
  }
}

impl<T: Std430 + ShaderSizedValueNodeType> DeviceParallelComputeIO<T>
  for DeviceMaterializeResult<T>
{
  fn materialize_storage_buffer(
    &self,
    _: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    self.clone()
  }
}

pub struct WriteIntoStorageWriter<T: Std430> {
  pub inner: Box<dyn DeviceInvocationComponent<Node<T>>>,
  pub result_write_idx: Arc<dyn Fn(Node<u32>) -> Node<u32>>,
  pub result_write_idx_hasher: Box<dyn ShaderHashProvider>,
  pub output: StorageBufferDataView<[T]>,
}

impl<T: Std430> ShaderHashProvider for WriteIntoStorageWriter<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.inner.hash_pipeline_with_type_info(hasher);
    self
      .result_write_idx_hasher
      .hash_pipeline_with_type_info(hasher);
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
    let output = builder.bind_by(&self.output);
    let result_write_idx_mapper = self.result_write_idx.clone();

    self
      .inner
      .build_shader(builder)
      .adhoc_invoke_with_self_size(move |inner, id| {
        let r = inner.invocation_logic(id);

        if_by(r.1.and(id.x().less_than(output.array_length())), || {
          let target_idx = result_write_idx_mapper(id.x());
          output.index(target_idx).store(r.0);
        });

        r
      })
      .into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.output);
    self.inner.bind_input(builder);
  }

  fn work_size(&self) -> Option<u32> {
    self.inner.work_size()
  }
}

pub fn custom_write_into_storage_buffer<T: Std430 + ShaderSizedValueNodeType>(
  source: &(impl DeviceParallelComputeIO<T> + ?Sized),
  cx: &mut DeviceParallelComputeCtx,
  write_position_mapper: impl Fn(Node<u32>) -> Node<u32> + 'static,
  result_write_idx_hasher: Box<dyn ShaderHashProvider>,
  write_target: StorageBufferDataView<[T]>,
) -> DeviceMaterializeResult<T> {
  let input_source = source.execute_and_expose(cx);

  assert!(write_target.item_count() >= source.result_size());

  let write = WriteIntoStorageWriter {
    inner: input_source,
    result_write_idx: Arc::new(write_position_mapper),
    output: write_target,
    result_write_idx_hasher,
  };

  let size = write.dispatch_compute(cx);

  DeviceMaterializeResult {
    buffer: write.output.into_readonly_view(),
    size,
  }
}

pub struct LinearWriterHash;
impl ShaderHashProvider for LinearWriterHash {
  shader_hash_type_id! {}
}

pub fn do_write_into_storage_buffer<T: Std430 + ShaderSizedValueNodeType>(
  source: &(impl DeviceParallelComputeIO<T> + ?Sized),
  cx: &mut DeviceParallelComputeCtx,
  write_target: StorageBufferDataView<[T]>,
) -> DeviceMaterializeResult<T> {
  custom_write_into_storage_buffer(source, cx, |x| x, Box::new(LinearWriterHash), write_target)
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
    self.materialize_storage_buffer(cx).into_boxed()
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
  ) -> DeviceMaterializeResult<T> {
    self.inner.materialize_storage_buffer(cx)
  }
  fn materialize_storage_buffer_into(
    &self,
    target: StorageBufferDataView<[T]>,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    self.inner.materialize_storage_buffer_into(target, cx)
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
    let (device_result, size, host_result) =
      pollster::block_on(self.inner.read_back_host(cx)).unwrap();

    println!("{} content is: {:?}", self.label, host_result);
    if let Some(size) = size {
      println!("{} has device size: {}", self.label, size);
    }

    // todo, log should not has any side effect, but we can not return the inner execute_and_expose
    // because forker expect the execute_and_expose is consumed once
    device_result.into_boxed()
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
