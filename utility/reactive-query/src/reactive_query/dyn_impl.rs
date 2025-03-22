use crate::*;

pub type BoxedDynReactiveQuery<K, V> = Box<dyn DynReactiveQuery<Key = K, Value = V>>;
pub type DynReactiveQueryPoll<K, V> = (BoxedDynQuery<K, ValueChange<V>>, BoxedDynQuery<K, V>);

pub trait DynQueryCompute: Sync + Send + 'static {
  type Key: CKey;
  type Value: CValue;
  fn resolve_dyn(&mut self) -> DynReactiveQueryPoll<Self::Key, Self::Value>;
}
impl<T: QueryCompute> DynQueryCompute for T {
  type Key = T::Key;
  type Value = T::Value;
  fn resolve_dyn(&mut self) -> DynReactiveQueryPoll<Self::Key, Self::Value> {
    let (d, v) = self.resolve();
    (Box::new(d), Box::new(v))
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

  fn resolve(&mut self) -> (Self::Changes, Self::View) {
    self.deref_mut().resolve_dyn()
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
