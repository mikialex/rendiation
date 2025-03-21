use crate::*;

pub type BoxedDynReactiveQuery<K, V> = Box<dyn DynReactiveQuery<Key = K, Value = V>>;
pub type DynReactiveQueryPoll<K, V> = (BoxedDynQuery<K, ValueChange<V>>, BoxedDynQuery<K, V>);

pub trait DynReactiveQuery: Sync + Send + 'static {
  type Key: CKey;
  type Value: CValue;
  fn poll_changes_dyn(&self, cx: &mut Context) -> DynReactiveQueryPoll<Self::Key, Self::Value>;

  fn extra_request_dyn(&mut self, request: &mut ReactiveQueryRequest);
}

impl<T: ReactiveQuery> DynReactiveQuery for T {
  type Key = T::Key;
  type Value = T::Value;
  fn poll_changes_dyn(&self, cx: &mut Context) -> DynReactiveQueryPoll<Self::Key, Self::Value> {
    let (d, v) = self.poll_changes(cx).resolve();
    (Box::new(d), Box::new(v))
  }

  fn extra_request_dyn(&mut self, request: &mut ReactiveQueryRequest) {
    self.request(request)
  }
}

impl<K: CKey, V: CValue> ReactiveQuery for BoxedDynReactiveQuery<K, V> {
  type Key = K;
  type Value = V;
  type Compute = DynReactiveQueryPoll<K, V>;

  fn poll_changes(&self, cx: &mut Context) -> Self::Compute {
    self.deref().poll_changes_dyn(cx)
  }
  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.deref_mut().extra_request_dyn(request)
  }
}
