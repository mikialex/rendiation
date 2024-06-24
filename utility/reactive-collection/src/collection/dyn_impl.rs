use crate::*;

pub trait DynReactiveCollection<K: CKey, V: CValue>: Sync + Send + 'static {
  fn poll_changes_dyn(
    &self,
    cx: &mut Context,
  ) -> (
    Box<dyn DynVirtualCollection<K, ValueChange<V>>>,
    Box<dyn DynVirtualCollection<K, V>>,
  );

  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation);
}

impl<K: CKey, V: CValue, T: ReactiveCollection<K, V>> DynReactiveCollection<K, V> for T {
  fn poll_changes_dyn(
    &self,
    cx: &mut Context,
  ) -> (
    Box<dyn DynVirtualCollection<K, ValueChange<V>>>,
    Box<dyn DynVirtualCollection<K, V>>,
  ) {
    let (d, v) = self.poll_changes(cx);
    (Box::new(d), Box::new(v))
  }

  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation) {
    self.extra_request(request)
  }
}

impl<K: CKey, V: CValue> ReactiveCollection<K, V> for Box<dyn DynReactiveCollection<K, V>> {
  type Changes = Box<dyn DynVirtualCollection<K, ValueChange<V>>>;
  type View = Box<dyn DynVirtualCollection<K, V>>;
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    self.deref().poll_changes_dyn(cx)
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.deref_mut().extra_request_dyn(request)
  }
}
