use crate::*;

mod self_contain;
pub use self_contain::*;

mod dyn_impl;
pub use dyn_impl::*;

mod operator;
pub use operator::*;

pub enum ExtraCollectionOperation {
  MemoryShrinkToFit,
}

pub trait ReactiveCollection<K: CKey, V: CValue>: Sync + Send + 'static {
  type Changes: VirtualCollection<K, ValueChange<V>>;
  type View: VirtualCollection<K, V>;
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View);

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation);
}

#[derive(Clone)]
pub struct CollectionPreviousView<'a, K, V> {
  current: &'a dyn DynVirtualCollection<K, V>,
  delta: &'a dyn DynVirtualCollection<K, ValueChange<V>>,
}
pub fn make_previous<'a, K, V>(
  current: &'a dyn DynVirtualCollection<K, V>,
  delta: &'a dyn DynVirtualCollection<K, ValueChange<V>>,
) -> CollectionPreviousView<'a, K, V> {
  CollectionPreviousView { current, delta }
}

/// the impl access the previous V
impl<'a, K: CKey, V: CValue> VirtualCollection<K, V> for CollectionPreviousView<'a, K, V> {
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    let current_not_changed = self
      .current
      .iter_key_value()
      .filter(|(k, _)| !self.delta.contains(k));

    let current_changed = self
      .delta
      .iter_key_value()
      .filter_map(|(k, v)| v.old_value().map(|v| (k, v.clone())));
    current_not_changed.chain(current_changed)
  }

  fn access(&self, key: &K) -> Option<V> {
    if let Some(change) = self.delta.access(key) {
      change.old_value().cloned()
    } else {
      self.current.access_dyn(key)
    }
  }
}

impl<K: CKey, V: CValue> ReactiveCollection<K, V> for () {
  type Changes = ();
  type View = ();
  fn poll_changes(&self, _: &mut Context) -> (Self::Changes, Self::View) {
    ((), ())
  }
  fn extra_request(&mut self, _: &mut ExtraCollectionOperation) {}
}
