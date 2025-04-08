use crate::*;

pub type BoxedDynReactiveQuery<K, V> = Box<dyn DynReactiveQuery<Key = K, Value = V>>;
pub type DynReactiveQueryPoll<K, V> = (BoxedDynQuery<K, ValueChange<V>>, BoxedDynQuery<K, V>);

pub trait DynQueryCompute: Sync + Send + 'static {
  type Key: CKey;
  type Value: CValue;
  fn resolve_dyn(&mut self, cx: &QueryResolveCtx) -> DynReactiveQueryPoll<Self::Key, Self::Value>;
  fn create_task_dyn(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> Pin<Box<dyn Send + Sync + Future<Output = DynReactiveQueryPoll<Self::Key, Self::Value>>>>;
}
impl<T: AsyncQueryCompute> DynQueryCompute for T {
  type Key = T::Key;
  type Value = T::Value;
  fn resolve_dyn(&mut self, cx: &QueryResolveCtx) -> DynReactiveQueryPoll<Self::Key, Self::Value> {
    let (d, v) = self.resolve(cx);
    (Box::new(d), Box::new(v))
  }
  fn create_task_dyn(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> Pin<Box<dyn Send + Sync + Future<Output = DynReactiveQueryPoll<Self::Key, Self::Value>>>> {
    let c = cx.resolve_cx().clone();
    self
      .create_task(cx)
      .map(move |mut r| r.resolve_dyn(&c))
      .into_boxed_future()
  }
}
pub type BoxedDynQueryCompute<K, V> = Box<dyn DynQueryCompute<Key = K, Value = V>>;

pub trait DynReactiveQuery: Sync + Send + 'static {
  type Key: CKey;
  type Value: CValue;
  fn poll_changes_dyn(&self, cx: &mut Context) -> BoxedDynQueryCompute<Self::Key, Self::Value>;

  fn extra_request_dyn(&mut self, request: &mut ReactiveQueryRequest);
}

impl<T: ReactiveQuery> DynReactiveQuery for T {
  type Key = T::Key;
  type Value = T::Value;
  fn poll_changes_dyn(&self, cx: &mut Context) -> BoxedDynQueryCompute<Self::Key, Self::Value> {
    Box::new(self.describe(cx))
  }

  fn extra_request_dyn(&mut self, request: &mut ReactiveQueryRequest) {
    self.request(request)
  }
}

impl<K: CKey, V: CValue> QueryCompute for BoxedDynQueryCompute<K, V> {
  type Key = K;
  type Value = V;
  type Changes = BoxedDynQuery<K, ValueChange<V>>;
  type View = BoxedDynQuery<K, V>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    self.deref_mut().resolve_dyn(cx)
  }
}
impl<K: CKey, V: CValue> AsyncQueryCompute for BoxedDynQueryCompute<K, V> {
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    self.create_task_dyn(cx)
  }
}

impl<K: CKey, V: CValue> ReactiveQuery for BoxedDynReactiveQuery<K, V> {
  type Key = K;
  type Value = V;
  type Compute = BoxedDynQueryCompute<K, V>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    self.deref().poll_changes_dyn(cx)
  }
  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.deref_mut().extra_request_dyn(request)
  }
}
