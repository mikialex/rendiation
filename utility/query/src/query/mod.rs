use std::any::Any;

use crate::*;

mod dyn_impl;
pub use dyn_impl::*;

mod operator;
pub use operator::*;

mod self_contain;
pub use self_contain::*;

pub type QueryMaterialized<K, V> = FastHashMap<K, V>;

pub trait Query: Send + Sync + Clone {
  type Key: CKey;
  type Value: CValue;
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_;
  fn access(&self, key: &Self::Key) -> Option<Self::Value>;
  fn contains(&self, key: &Self::Key) -> bool {
    self.access(key).is_some()
  }

  /// the implementation allows to have false positive(return has item but is actually empty)
  fn has_item_hint(&self) -> bool;

  fn materialize(&self) -> Arc<QueryMaterialized<Self::Key, Self::Value>> {
    Arc::new(self.iter_key_value().collect())
  }
}

impl<T: Query> Query for &T {
  type Key = T::Key;
  type Value = T::Value;
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    (*self).iter_key_value()
  }

  fn access(&self, k: &Self::Key) -> Option<Self::Value> {
    (*self).access(k)
  }

  fn has_item_hint(&self) -> bool {
    (*self).has_item_hint()
  }
}

impl<T: Query> Query for Option<T> {
  type Key = T::Key;
  type Value = T::Value;
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self.iter().flat_map(|v| v.iter_key_value())
  }
  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    self.as_ref().and_then(|v| v.access(key))
  }

  fn has_item_hint(&self) -> bool {
    self.as_ref().is_some_and(|v| v.has_item_hint())
  }
}

pub struct EmptyQuery<K, V>(PhantomData<(K, V)>);

impl<K, V> Clone for EmptyQuery<K, V> {
  fn clone(&self) -> Self {
    Self(self.0)
  }
}

impl<K, V> Default for EmptyQuery<K, V> {
  fn default() -> Self {
    Self(PhantomData)
  }
}

impl<K: CKey, V: CValue> Query for EmptyQuery<K, V> {
  type Key = K;
  type Value = V;
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    std::iter::empty()
  }

  fn access(&self, _: &K) -> Option<V> {
    None
  }

  fn has_item_hint(&self) -> bool {
    false
  }
}

impl<K: CKey, V: CValue> Query for Arc<FastHashMap<K, V>> {
  type Key = K;
  type Value = V;

  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    self.iter().map(|(k, v)| (k.clone(), v.clone()))
  }

  fn access(&self, key: &K) -> Option<V> {
    self.get(key).cloned()
  }
  fn materialize(&self) -> Arc<FastHashMap<K, V>> {
    self.clone()
  }

  fn has_item_hint(&self) -> bool {
    !self.is_empty()
  }
}

impl<K: CKey, V: CValue> Query for FastHashMap<K, V> {
  type Key = K;
  type Value = V;

  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    self.iter().map(|(k, v)| (k.clone(), v.clone()))
  }

  fn access(&self, key: &K) -> Option<V> {
    self.get(key).cloned()
  }

  fn has_item_hint(&self) -> bool {
    !self.is_empty()
  }
}

impl<V: CValue> Query for Arena<V> {
  type Key = u32;
  type Value = V;

  fn iter_key_value(&self) -> impl Iterator<Item = (u32, V)> + '_ {
    self.iter().map(|(h, v)| (h.index() as u32, v.clone()))
  }

  fn access(&self, key: &u32) -> Option<V> {
    let handle = self.get_handle(*key as usize).unwrap();
    self.get(handle).cloned()
  }

  fn has_item_hint(&self) -> bool {
    !self.is_empty()
  }
}

impl<V: CValue> Query for IndexReusedVec<V> {
  type Key = u32;
  type Value = V;

  fn iter_key_value(&self) -> impl Iterator<Item = (u32, V)> + '_ {
    self.iter().map(|(k, v)| (k, v.clone()))
  }

  fn access(&self, key: &u32) -> Option<V> {
    self.try_get(*key).cloned()
  }

  fn has_item_hint(&self) -> bool {
    !self.is_empty()
  }
}

impl<V: CValue> Query for IndexKeptVec<V> {
  type Key = u32;
  type Value = V;

  fn iter_key_value(&self) -> impl Iterator<Item = (u32, V)> + '_ {
    self.iter().map(|(k, v)| (k as u32, v.clone()))
  }

  fn access(&self, key: &u32) -> Option<V> {
    self.try_get(key.alloc_index() as usize).cloned()
  }

  fn has_item_hint(&self) -> bool {
    !self.is_empty()
  }
}

#[derive(Clone)]
pub struct IdenticalCollection<V> {
  pub value: V,
  pub size: u32,
}

impl<V: CValue> Query for IdenticalCollection<V> {
  type Key = u32;
  type Value = V;
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, V)> + '_ {
    std::iter::repeat_n(self.value.clone(), self.size as usize)
      .enumerate()
      .map(|(id, v)| (id as u32, v))
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    if key < &self.size {
      Some(self.value.clone())
    } else {
      None
    }
  }

  fn has_item_hint(&self) -> bool {
    self.size > 0
  }
}

#[derive(Clone)]
pub struct KeptQuery<T> {
  pub query: T,
  pub holder: Arc<dyn Any + Send + Sync>,
}

impl<T: Query> Query for KeptQuery<T> {
  type Key = T::Key;
  type Value = T::Value;

  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self.query.iter_key_value()
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    self.query.access(key)
  }

  fn has_item_hint(&self) -> bool {
    self.query.has_item_hint()
  }
}

impl<T: DynValueRefQuery> DynValueRefQuery for KeptQuery<T>
where
  Self: DynQuery<Key = T::Key, Value = T::Value>,
{
  fn access_ref(&self, key: &Self::Key) -> Option<&Self::Value> {
    self.query.access_ref(key)
  }
}

impl<T: MultiQuery> MultiQuery for KeptQuery<T> {
  type Key = T::Key;
  type Value = T::Value;

  fn iter_keys(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self.query.iter_keys()
  }

  fn access_multi(&self, key: &Self::Key) -> Option<impl Iterator<Item = Self::Value> + '_> {
    self.query.access_multi(key)
  }
}
