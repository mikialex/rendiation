use std::mem::ManuallyDrop;

use crate::*;

pub struct ComputeResultForker<T: Std430> {
  pub inner: Box<dyn DeviceParallelComputeIO<T>>,
  /// if we not add cache here, the cost may be exponential!
  pub size_cache: u32,
  pub children: RwLock<FastHashMap<usize, ComputeResultForkerInstance<T>>>,
  pub id: RwLock<usize>,
}

// todo mem leak
pub struct ComputeResultForkerInstance<T: Std430> {
  pub upstream: Arc<ComputeResultForker<T>>,
  pub result: Arc<RwLock<Option<DeviceMaterializeResult<T>>>>,
  pub id: usize,
}

impl<T: Std430> ComputeResultForkerInstance<T> {
  pub fn from_upstream(upstream: Box<dyn DeviceParallelComputeIO<T>>) -> Self {
    let forker = ComputeResultForker {
      size_cache: upstream.result_size(),
      inner: upstream,
      children: Default::default(),
      id: RwLock::new(0),
    };
    let r = Self {
      upstream: Arc::new(forker),
      result: Default::default(),
      id: 0,
    };

    r.upstream.children.write().insert(0, r.link_clone());
    r
  }

  fn link_clone(&self) -> Self {
    Self {
      upstream: self.upstream.clone(),
      result: self.result.clone(),
      id: self.id,
    }
  }
}

impl<T: Std430> Clone for ComputeResultForkerInstance<T> {
  fn clone(&self) -> Self {
    {
      *self.upstream.id.write() += 1;
    }

    let id = *self.upstream.id.read();
    let r = Self {
      upstream: self.upstream.clone(),
      result: Default::default(),
      id,
    };

    self.upstream.children.write().insert(id, r.link_clone());

    r
  }
}
impl<T: Std430> Drop for ComputeResultForkerInstance<T> {
  fn drop(&mut self) {
    let a = self.upstream.children.write().remove(&self.id);
    let _ = ManuallyDrop::new(a);
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
    self.materialize_storage_buffer(cx).into_boxed()
  }
  fn result_size(&self) -> u32 {
    self.upstream.size_cache
  }
}

impl<T> DeviceParallelComputeIO<T> for ComputeResultForkerInstance<T>
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
    if let Some(result) = self.result.write().take() {
      return result;
    }

    let result = self.upstream.inner.materialize_storage_buffer(cx);
    let children = self.upstream.children.read();
    for c in children.values() {
      let result = result.clone();
      if c.result.write().replace(result).is_some() {
        panic!("all forked result must be consumed")
      }
    }

    self.materialize_storage_buffer(cx)
  }
}

// #[should_panic]
// #[pollster::test]
// async fn test_not_full_consume() {
//   let input = vec![1_u32; 70];
//   let expect = input.clone();

//   let input = input.into_forker();
//   let input2 = input.clone();

//   input.run_test(&expect).await;
//   input2.run_test(&expect).await;
// }

#[pollster::test]
async fn test() {
  let input = vec![1_u32; 70];

  let expect = vec![2_u32; 70];

  let input = input.into_forker();
  let input2 = input.clone();

  input
    .zip(input2)
    .map(|(a, b)| a + b)
    .run_test(&expect)
    .await
}
