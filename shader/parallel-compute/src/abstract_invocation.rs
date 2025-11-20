use crate::*;

/// abstract device invocation. the invocation cost should only exist if user has called
///  `invocation_logic`, as well as invocation_size.
pub trait DeviceInvocation<T> {
  // todo, we should separate check and access in different fn to avoid unnecessary check;
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (T, Node<bool>);

  fn invocation_size(&self) -> Node<Vec3<u32>>;

  fn end_point(&self) -> T {
    let clamp_target = self.invocation_size() - val(Vec3::one());
    self.invocation_logic(clamp_target).0
  }

  fn start_point(&self) -> T {
    self.invocation_logic(val(Vec3::zero())).0
  }
}

impl<T> DeviceInvocation<T> for Box<dyn DeviceInvocation<T>> {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (T, Node<bool>) {
    (**self).invocation_logic(logic_global_id)
  }
  fn invocation_size(&self) -> Node<Vec3<u32>> {
    (**self).invocation_size()
  }
}

pub trait DeviceInvocationExt<T>: DeviceInvocation<T> + 'static + Sized {
  fn into_boxed(self) -> Box<dyn DeviceInvocation<T>> {
    Box::new(self)
  }

  fn zip<U>(self, other: impl DeviceInvocation<U> + 'static) -> DeviceInvocationZip<T, U> {
    DeviceInvocationZip(self.into_boxed(), other.into_boxed())
  }

  fn adhoc_invoke_with_self_size<R>(
    self,
    r: impl Fn(&Self, Node<Vec3<u32>>) -> (R, Node<bool>) + 'static,
  ) -> impl DeviceInvocation<R>
  where
    R: Copy,
  {
    AdhocInvocationResult {
      upstream: self,
      phantom: PhantomData,
      compute: Box::new(r),
    }
  }
}
impl<T, X> DeviceInvocationExt<T> for X where X: DeviceInvocation<T> + 'static + Sized {}

pub struct RealAdhocInvocationResult<S, R> {
  pub inner: S,
  pub compute: Box<dyn Fn(&S, Node<Vec3<u32>>) -> (R, Node<bool>)>,
  pub size: Box<dyn Fn(&S) -> Node<Vec3<u32>>>,
}

impl<S, R> DeviceInvocation<R> for RealAdhocInvocationResult<S, R> {
  fn invocation_logic(&self, id: Node<Vec3<u32>>) -> (R, Node<bool>) {
    (self.compute)(&self.inner, id)
  }
  fn invocation_size(&self) -> Node<Vec3<u32>> {
    (self.size)(&self.inner)
  }
}

/// i think this is a mistake
pub struct AdhocInvocationResult<S, T, R> {
  upstream: S,
  phantom: PhantomData<T>,
  compute: Box<dyn Fn(&S, Node<Vec3<u32>>) -> (R, Node<bool>)>,
}

impl<S: DeviceInvocation<T>, T, R> DeviceInvocation<R> for AdhocInvocationResult<S, T, R> {
  fn invocation_logic(&self, id: Node<Vec3<u32>>) -> (R, Node<bool>) {
    (self.compute)(&self.upstream, id)
  }
  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.upstream.invocation_size()
  }
}

pub fn compute_dispatch_size(work_size: u32, workgroup_size: u32) -> u32 {
  work_size.div_ceil(workgroup_size)
}
pub fn device_compute_dispatch_size(work_size: Node<u32>, workgroup_size: Node<u32>) -> Node<u32> {
  (work_size + workgroup_size - val(1)) / workgroup_size
}
