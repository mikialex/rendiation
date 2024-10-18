use crate::*;

pub type BoxedDynReactiveCollection<K, V> = Box<dyn DynReactiveCollection<Key = K, Value = V>>;
pub type DynReactiveCollectionPoll<K, V> = (
  BoxedDynVirtualCollection<K, ValueChange<V>>,
  BoxedDynVirtualCollection<K, V>,
);

pub trait DynReactiveCollection: Sync + Send + 'static {
  type Key: CKey;
  type Value: CValue;
  fn poll_changes_dyn(&self, cx: &mut Context)
    -> DynReactiveCollectionPoll<Self::Key, Self::Value>;

  fn extra_request_dyn(&mut self, request: &mut ReactiveCollectionRequest);
}

impl<T: ReactiveCollection> DynReactiveCollection for T {
  type Key = T::Key;
  type Value = T::Value;
  fn poll_changes_dyn(
    &self,
    cx: &mut Context,
  ) -> DynReactiveCollectionPoll<Self::Key, Self::Value> {
    let (d, v) = self.poll_changes(cx);
    (Box::new(d), Box::new(v))
  }

  fn extra_request_dyn(&mut self, request: &mut ReactiveCollectionRequest) {
    self.request(request)
  }
}

impl<K: CKey, V: CValue> ReactiveCollection for BoxedDynReactiveCollection<K, V> {
  type Key = K;
  type Value = V;
  type Changes = BoxedDynVirtualCollection<K, ValueChange<V>>;
  type View = BoxedDynVirtualCollection<K, V>;
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    self.deref().poll_changes_dyn(cx)
  }
  fn request(&mut self, request: &mut ReactiveCollectionRequest) {
    self.deref_mut().extra_request_dyn(request)
  }
}
