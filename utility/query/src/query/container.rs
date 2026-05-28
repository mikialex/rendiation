use std::any::Any;

use crate::*;

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

#[test]
fn test_empty_query() {
  let q: EmptyQuery<u32, String> = EmptyQuery::default();
  super::operator::validate_query_consistency(&q);
  assert_eq!(q.access(&1), None);
}

#[test]
fn test_empty_multi_query() {
  let q: EmptyQuery<u32, String> = EmptyQuery::default();
  super::multi_query::validate_multi_query_consistency(&q);
  assert!(q.access_multi(&1).is_none());
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

#[test]
fn test_arc_fast_hash_map_query() {
  let mut base = FastHashMap::default();
  base.insert(1u32, "a".to_string());
  base.insert(2, "b".to_string());
  let q = Arc::new(base);

  super::operator::validate_query_consistency(&q);
  assert_eq!(q.access(&1), Some("a".to_string()));
  assert_eq!(q.access(&2), Some("b".to_string()));
  assert_eq!(q.access(&3), None);
}

impl<K: CKey> Query for FastHashSet<K> {
  type Key = K;
  type Value = ();

  fn iter_key_value(&self) -> impl Iterator<Item = (K, ())> + '_ {
    self.iter().map(|k| (k.clone(), ()))
  }

  fn access(&self, key: &K) -> Option<()> {
    self.contains(key).then_some(())
  }

  fn has_item_hint(&self) -> bool {
    !self.is_empty()
  }
}

#[test]
fn test_fast_hash_set_query() {
  let mut set = FastHashSet::default();
  set.insert(1u32);
  set.insert(2);
  set.insert(3);

  super::operator::validate_query_consistency(&set);
  assert_eq!(set.access(&1), Some(()));
  assert_eq!(set.access(&2), Some(()));
  assert_eq!(set.access(&4), None);
}

#[test]
fn test_fast_hash_set_empty_query() {
  let set: FastHashSet<u32> = FastHashSet::default();
  super::operator::validate_query_consistency(&set);
  assert_eq!(set.access(&1), None);
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

#[test]
fn test_fast_hash_map_query() {
  let mut map = FastHashMap::default();
  map.insert(1u32, "hello".to_string());
  map.insert(2, "world".to_string());

  super::operator::validate_query_consistency(&map);
  assert_eq!(map.access(&1), Some("hello".to_string()));
  assert_eq!(map.access(&2), Some("world".to_string()));
  assert_eq!(map.access(&3), None);
}

impl<V: CValue> Query for Arena<V> {
  type Key = u32;
  type Value = V;

  fn iter_key_value(&self) -> impl Iterator<Item = (u32, V)> + '_ {
    self.iter().map(|(h, v)| (h.index() as u32, v.clone()))
  }

  fn access(&self, key: &u32) -> Option<V> {
    let handle = self.get_handle(*key as usize)?;
    self.get(handle).cloned()
  }

  fn has_item_hint(&self) -> bool {
    !self.is_empty()
  }
}

#[test]
fn test_arena_query() {
  let mut arena = Arena::new();
  let h0 = arena.insert(10i32);
  let h1 = arena.insert(20);
  let h2 = arena.insert(30);

  let k0 = h0.index() as u32;
  let k1 = h1.index() as u32;
  let k2 = h2.index() as u32;

  super::operator::validate_query_consistency(&arena);
  assert_eq!(arena.access(&k0), Some(10));
  assert_eq!(arena.access(&k1), Some(20));
  assert_eq!(arena.access(&k2), Some(30));
  assert_eq!(arena.access(&(k2 + 1)), None);
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

#[test]
fn test_index_reused_vec_query() {
  let mut vec = IndexReusedVec::default();
  let k0 = vec.insert(10i32);
  let k1 = vec.insert(20);
  let k2 = vec.insert(30);

  super::operator::validate_query_consistency(&vec);
  assert_eq!(vec.access(&k0), Some(10));
  assert_eq!(vec.access(&k1), Some(20));
  assert_eq!(vec.access(&k2), Some(30));
}

#[test]
fn test_index_reused_vec_with_removal_query() {
  let mut vec = IndexReusedVec::default();
  let k0 = vec.insert(10i32);
  vec.insert(20);
  vec.remove(k0);
  let k2 = vec.insert(30); // reuses k0's slot

  super::operator::validate_query_consistency(&vec);
  assert_eq!(vec.access(&k2), Some(30));
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

#[test]
fn test_index_kept_vec_query() {
  let mut vec = IndexKeptVec::default();
  vec.insert(0, 10i32);
  vec.insert(1, 20);
  vec.insert(5, 30);

  super::operator::validate_query_consistency(&vec);
  assert_eq!(vec.access(&0), Some(10));
  assert_eq!(vec.access(&1), Some(20));
  assert_eq!(vec.access(&5), Some(30));
  assert_eq!(vec.access(&2), None);
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

#[test]
fn test_identical_collection_query() {
  let c = IdenticalCollection {
    value: "same".to_string(),
    size: 3,
  };

  super::operator::validate_query_consistency(&c);
  assert_eq!(c.access(&0), Some("same".to_string()));
  assert_eq!(c.access(&1), Some("same".to_string()));
  assert_eq!(c.access(&2), Some("same".to_string()));
  assert_eq!(c.access(&3), None);
}

#[test]
fn test_identical_collection_empty_query() {
  let c = IdenticalCollection {
    value: 42i32,
    size: 0,
  };

  super::operator::validate_query_consistency(&c);
  assert_eq!(c.access(&0), None);
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

#[test]
fn test_kept_query() {
  let mut base = FastHashMap::default();
  base.insert(1u32, "hello".to_string());

  let kept = KeptQuery {
    query: base,
    holder: Arc::new(42i32),
  };

  super::operator::validate_query_consistency(&kept);
  assert_eq!(kept.access(&1), Some("hello".to_string()));
  assert_eq!(kept.access(&2), None);
}
