use crate::*;

pub struct ComputeResultForker<T: Std430> {
  pub inner: Box<dyn DeviceParallelComputeIO<T>>,
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
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    if let Some(result) = self.result.write().take() {
      return Box::new(StorageBufferReadOnlyDataViewReadIntoShader(result));
    }

    let result = self.upstream.inner.materialize_storage_buffer(cx);
    let children = self.upstream.children.read();
    for c in children.iter() {
      let result = result.clone();
      if c.result.write().replace(result).is_some() {
        panic!("all forked result must be consumed")
      }
    }

    self.execute_and_expose(cx)
  }

  fn work_size(&self) -> u32 {
    self.upstream.inner.work_size()
  }
}

impl<T> DeviceParallelComputeIO<T> for ComputeResultForkerInstance<T>
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
    if let Some(result) = self.result.write().take() {
      return result;
    }

    let result = self.upstream.inner.materialize_storage_buffer(cx);
    let children = self.upstream.children.read();
    for c in children.iter() {
      let result = result.clone();
      if c.result.write().replace(result).is_some() {
        panic!("all forked result must be consumed")
      }
    }

    self.materialize_storage_buffer(cx)
  }
}
