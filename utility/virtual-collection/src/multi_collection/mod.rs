use crate::*;

mod dyn_impl;
pub use dyn_impl::*;

mod operator;
pub use operator::*;

pub trait VirtualMultiCollection: Send + Sync + Clone {
  type Key: CKey;
  type Value: CValue;
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = Self::Key> + '_;
  /// if k is not in the collection at all, return None.
  /// if k is in the collection but map to none of v, return empty iterator
  fn access_multi(&self, key: &Self::Key) -> Option<impl Iterator<Item = Self::Value> + '_>;
  fn access_multi_value(&self, key: &Self::Key) -> impl Iterator<Item = Self::Value> + '_ {
    self
      .access_multi(key)
      .map(|v| Box::new(v) as Box<dyn Iterator<Item = Self::Value> + '_>) // todo impl iterator for better performance
      .unwrap_or_else(|| Box::new(std::iter::empty()))
  }

  fn access_multi_visitor(&self, key: &Self::Key, visitor: &mut dyn FnMut(Self::Value)) {
    if let Some(v) = self.access_multi(key) {
      for v in v {
        visitor(v);
      }
    }
  }
}

impl<K: CKey, V: CKey> VirtualMultiCollection for EmptyCollection<K, V> {
  type Key = K;
  type Value = V;
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K> + '_ {
    std::iter::empty()
  }

  fn access_multi(&self, _: &K) -> Option<impl Iterator<Item = V> + '_> {
    None::<std::iter::Empty<V>>
  }
}
