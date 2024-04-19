use std::sync::Arc;

use parking_lot::RwLock;

use crate::*;

impl<T: ShaderNodeType> DeviceInvocation<Node<T>> for Node<ShaderReadOnlyStoragePtr<[T]>> {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<T>, Node<bool>) {
    let idx = logic_global_id.x();
    let r = idx.less_than(self.array_length());
    (r.select(self.index(idx).load(), zero_shader_value()), r)
  }
}

impl<T> DeviceParallelCompute<Node<T>> for StorageBufferReadOnlyDataView<[T]>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn compute_result(
    &self,
    _cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationBuilder<Node<T>>> {
    Box::new(StorageBufferReadOnlyDataViewReadIntoShader(self.clone()))
  }

  fn work_size(&self) -> u32 {
    let size: u64 = self.view_byte_size().into();
    let count = size / std::mem::size_of::<T>() as u64;
    count as u32
  }
}

struct StorageBufferReadOnlyDataViewReadIntoShader<T: Std430>(StorageBufferReadOnlyDataView<[T]>);

impl<T> DeviceInvocationBuilder<Node<T>> for StorageBufferReadOnlyDataViewReadIntoShader<T>
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

pub(crate) fn write_into_storage_buffer<T: Std430 + ShaderSizedValueNodeType>(
  source: &impl DeviceParallelCompute<Node<T>>,
  cx: &mut DeviceParallelComputeCtx,
) -> StorageBufferDataView<[T]> {
  let group_size = 256;
  let input_source = source.compute_result(cx);

  let output = create_gpu_read_write_storage::<[T]>(source.work_size() as usize, &cx.gpu);

  let pipeline = cx.get_or_create_compute_pipeline(&input_source, |cx| {
    cx.config_work_group_size(group_size);
    let source = input_source.build_shader(cx.0);
    let output = cx.bind_by(&output);

    let (r, valid) = source.invocation_logic(cx.global_invocation_id());

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
  pub inner: Box<dyn DeviceParallelCompute<Node<T>>>,
}

impl<T: ShaderSizedValueNodeType + Std430> DeviceParallelCompute<Node<T>>
  for WriteStorageReadBack<T>
{
  fn compute_result(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationBuilder<Node<T>>> {
    let temp_result = write_into_storage_buffer(&self.inner, cx).into_readonly_view();
    Box::new(StorageBufferReadOnlyDataViewReadIntoShader(temp_result))
  }

  // // faster path, avoid extra read back
  // fn write_into_storage_buffer(
  //   &self,
  //   cx: &mut DeviceParallelComputeCtx,
  // ) -> StorageBufferDataView<[T]> {
  //   write_into_storage_buffer(&self.inner, cx)
  // }

  fn work_size(&self) -> u32 {
    self.inner.work_size()
  }
}

pub struct ComputeResultForker<T: Std430> {
  pub inner: Box<dyn DeviceParallelCompute<Node<T>>>,
  pub children: RwLock<Vec<ComputeResultForkerInstance<T>>>,
}

pub struct ComputeResultForkerInstance<T: Std430> {
  pub upstream: Arc<ComputeResultForker<T>>,
  pub result: Arc<RwLock<Option<StorageBufferReadOnlyDataView<[T]>>>>,
}

impl<T: Std430> Clone for ComputeResultForkerInstance<T> {
  fn clone(&self) -> Self {
    Self {
      upstream: self.upstream.clone(),
      result: self.result.clone(),
    }
  }
}

impl<T> DeviceParallelCompute<Node<T>> for ComputeResultForkerInstance<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn compute_result(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationBuilder<Node<T>>> {
    if let Some(result) = self.result.write().take() {
      return Box::new(StorageBufferReadOnlyDataViewReadIntoShader(result));
    }

    let result = self.upstream.inner.write_into_storage_buffer(cx);
    let children = self.upstream.children.read();
    for c in children.iter() {
      let result = result.clone().into_readonly_view();
      if c.result.write().replace(result).is_some() {
        panic!("all forked result must be consumed")
      }
    }

    self.compute_result(cx)
  }

  fn work_size(&self) -> u32 {
    self.upstream.inner.work_size()
  }
}
