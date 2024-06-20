use crate::*;

mod operator;
pub use operator::*;

pub trait VirtualMultiCollection<K, V: CValue>: Send + Sync + DynClone {
  fn iter_key_in_multi_collection(&self) -> Box<dyn Iterator<Item = K> + '_>;
  /// if k is not in the collection at all, return None.
  /// if k is in the collection but map to none of v, return empty iterator
  fn access_multi(&self, key: &K) -> Option<Box<dyn Iterator<Item = V> + '_>>;
  fn access_multi_value(&self, key: &K) -> Box<dyn Iterator<Item = V> + '_> {
    self
      .access_multi(key)
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
impl<'a, K, V> Clone for Box<dyn VirtualMultiCollection<K, V> + 'a> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

impl<'a, K: CKey, V: CKey> VirtualMultiCollection<K, V>
  for Box<dyn VirtualMultiCollection<K, V> + 'a>
{
  fn iter_key_in_multi_collection(&self) -> Box<dyn Iterator<Item = K> + '_> {
    (**self).iter_key_in_multi_collection()
  }

  fn access_multi(&self, key: &K) -> Option<Box<dyn Iterator<Item = V> + '_>> {
    (**self).access_multi(key)
  }
}

/// it's useful to use () as the empty collection
impl<K: CKey, V: CKey> VirtualMultiCollection<K, V> for () {
  fn iter_key_in_multi_collection(&self) -> Box<dyn Iterator<Item = K> + '_> {
    Box::new([].into_iter())
  }

  fn access_multi(&self, _: &K) -> Option<Box<dyn Iterator<Item = V> + '_>> {
    None
  }
}
