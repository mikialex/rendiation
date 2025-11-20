use crate::*;

impl<T: ShaderSizedValueNodeType> DeviceInvocation<Node<T>> for DynLengthArrayView<T> {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<T>, Node<bool>) {
    let idx = logic_global_id.x();
    let r = idx.less_than(self.array_length());
    let result = r.select_branched(|| self.index(idx).load(), zeroed_val);
    (result, r)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    (self.array_length(), val(0), val(0)).into()
  }
}

impl<T: ShaderSizedValueNodeType> DeviceInvocation<Node<T>> for DynLengthArrayReadonlyView<T> {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<T>, Node<bool>) {
    let idx = logic_global_id.x();
    let r = idx.less_than(self.array_length());
    let result = r.select_branched(|| self.index(idx).load(), zeroed_val);
    (result, r)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    (self.array_length(), val(0), val(0)).into()
  }
}

impl<T: ShaderSizedValueNodeType> DeviceInvocation<Node<T>>
  for (DynLengthArrayReadonlyView<T>, Node<Vec4<u32>>)
{
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<T>, Node<bool>) {
    let idx = logic_global_id.x();
    let r = idx.less_than(self.1.x());
    let result = r.select_branched(|| self.0.index(idx).load(), zeroed_val);
    (result, r)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.1.xyz()
  }
}

pub fn slice_into_compute<T: Std430 + ShaderSizedValueNodeType>(
  data: &[T],
  cx: &mut DeviceParallelComputeCtx,
) -> DeviceMaterializeResult<T> {
  let storage = create_gpu_readonly_storage(data, &cx.gpu);
  storage_full_into_compute(storage)
}

pub fn storage_full_into_compute<T: Std430 + ShaderSizedValueNodeType>(
  storage: StorageBufferReadonlyDataView<[T]>,
) -> DeviceMaterializeResult<T> {
  DeviceMaterializeResult::full_buffer(storage)
}

#[pollster::test]
async fn test_storage_buffer() {
  gpu_cx!(cx);
  let input = vec![1_u32; 70];
  let expect = input.clone();

  let input = slice_into_compute(&input, cx);

  input.run_test(cx, &expect).await
}

#[derive(Clone)]
pub struct DeviceMaterializeResult<T: Std430> {
  pub buffer: StorageBufferReadonlyDataView<[T]>,
  pub size: Option<StorageBufferReadonlyDataView<Vec4<u32>>>,
}

impl<T: Std430> DeviceMaterializeResult<T> {
  pub fn full_buffer(buffer: StorageBufferReadonlyDataView<[T]>) -> Self {
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
  fn result_size(&self) -> u32 {
    self.buffer.item_count()
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

  fn clone_boxed(&self) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    Box::new(self.clone())
  }
}
impl<T: Std430> ShaderHashProvider for DeviceMaterializeResult<T> {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.size.is_some().hash(hasher)
  }
}

impl<T: Std430 + ShaderSizedValueNodeType> DeviceInvocationComponentIO<T>
  for DeviceMaterializeResult<T>
{
  fn materialize_storage_buffer_into(
    &self,
    _target: StorageBufferDataView<[T]>,
    _cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    self.clone()
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct WriteIntoStorageWriter<T: Std430> {
  pub inner: Box<dyn DeviceInvocationComponent<Node<T>>>,
  pub result_write_idx: Arc<dyn Fn(Node<u32>) -> (Node<u32>, Node<bool>)>,
  pub result_write_idx_hasher: Arc<dyn ShaderHashProvider>,
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
  fn result_size(&self) -> u32 {
    self.inner.result_size()
  }

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
          let (target_idx, write) = result_write_idx_mapper(id.x());
          if_by(write, || {
            output.index(target_idx).store(r.0);
          });
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

  fn clone_boxed(&self) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    Box::new(self.clone())
  }
}

pub fn custom_write_into_storage_buffer<T: Std430 + ShaderSizedValueNodeType>(
  source: &(impl DeviceInvocationComponentIO<T> + ?Sized),
  cx: &mut DeviceParallelComputeCtx,
  write_position_mapper: impl Fn(Node<u32>) -> (Node<u32>, Node<bool>) + 'static,
  result_write_idx_hasher: Arc<dyn ShaderHashProvider>,
  write_target: StorageBufferDataView<[T]>,
) -> DeviceMaterializeResult<T> {
  assert!(write_target.item_count() >= source.result_size());

  let write = WriteIntoStorageWriter {
    inner: source.clone_boxed(),
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
  source: &(impl DeviceInvocationComponentIO<T> + ?Sized),
  cx: &mut DeviceParallelComputeCtx,
  write_target: StorageBufferDataView<[T]>,
) -> DeviceMaterializeResult<T> {
  custom_write_into_storage_buffer(
    source,
    cx,
    |x| (x, val(true)),
    Arc::new(LinearWriterHash),
    write_target,
  )
}
