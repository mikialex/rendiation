use crate::*;

mod dyn_impl;
pub use dyn_impl::*;

mod operator;
pub use operator::*;

pub trait VirtualMultiCollection<K, V: CValue>: Send + Sync + Clone {
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K> + '_;
  /// if k is not in the collection at all, return None.
  /// if k is in the collection but map to none of v, return empty iterator
  fn access_multi(&self, key: &K) -> Option<impl Iterator<Item = V> + '_>;
  fn access_multi_value(&self, key: &K) -> impl Iterator<Item = V> + '_ {
    self
      .access_multi(key)
      .map(|v| Box::new(v) as Box<dyn Iterator<Item = V> + '_>) // todo impl iterator for better performance
      .unwrap_or_else(|| Box::new(std::iter::empty()))
  }

  fn access_multi_visitor(&self, key: &K, visitor: &mut dyn FnMut(V)) {
    if let Some(v) = self.access_multi(key) {
      for v in v {
        visitor(v);
      }
    }
  }
}

/// it's useful to use () as the empty collection
impl<K: CKey, V: CKey> VirtualMultiCollection<K, V> for () {
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = K> + '_ {
    std::iter::empty()
  }

  fn access_multi(&self, _: &K) -> Option<impl Iterator<Item = V> + '_> {
    None::<std::iter::Empty<V>>
  }
}
