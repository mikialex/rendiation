mod delta_as_change;
pub use delta_as_change::*;

use crate::*;

/// Abstract batch change container
///
/// Why do we have this change abstract in addition to delta query?
/// - removing and mutation is separated, because removing likely to be consumed first to reduce peak memory usage
///   in downstream processing
/// - not care about(storing or procession) previous value to reduce memory usage and improve performance
/// - reduce code bloating
///
/// The call convention of this trait:
/// - always call [DataChanges::iter_removed] first and then call [DataChanges::iter_update_or_insert]
/// - keys can be duplicated in either removed or update_or_insert iteration
///   - the later witnessed value for same key is considered as the final synced value
///   - key can be in both removed and update_or_insert(it's ok because we do remove first)
///   - can remove none exist key(it's ok because it's not exist at all)
pub trait DataChanges: Send + Sync + Clone {
  type Key: CKey;
  type Value;
  fn has_change(&self) -> bool;
  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_;
  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_;

  fn materialize(self) -> Arc<LinearBatchChanges<Self::Key, Self::Value>> {
    Arc::new(LinearBatchChanges {
      removed: self.iter_removed().collect(),
      update_or_insert: self.iter_update_or_insert().collect(),
    })
  }

  fn map_changes_key<U: CKey>(
    self,
    f: impl Fn(Self::Key) -> U + Clone + Send + Sync + 'static,
  ) -> impl DataChanges<Key = U, Value = Self::Value> {
    MapChangesKey {
      base: self,
      mapper: f,
    }
  }

  fn collective_map<V: CValue>(
    self,
    f: impl Fn(Self::Value) -> V + Clone + Send + Sync + 'static,
  ) -> impl DataChanges<Key = Self::Key, Value = V> {
    DataChangesMap {
      base: self,
      mapper: f,
    }
  }

  fn collective_filter_map<V>(
    self,
    f: impl Fn(Self::Value) -> Option<V> + Clone + Send + Sync + 'static,
  ) -> impl DataChanges<Key = Self::Key, Value = V> {
    DataChangesFilterMap {
      base: self,
      mapper: f,
    }
  }

  fn collective_filter_kv_map<V>(
    self,
    f: impl Fn(&Self::Key, Self::Value) -> Option<V> + Clone + Send + Sync + 'static,
  ) -> impl DataChanges<Key = Self::Key, Value = V> {
    DataChangesFilterKVMap {
      base: self,
      mapper: f,
    }
  }
}

impl<K: CKey, V: CValue> DataChanges for EmptyQuery<K, V> {
  type Key = K;
  type Value = V;

  fn has_change(&self) -> bool {
    false
  }

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    std::iter::empty()
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    std::iter::empty()
  }
}

#[test]
fn test_empty_data_changes() {
  let q: EmptyQuery<u32, String> = EmptyQuery::default();
  assert!(!q.has_change());
  assert_eq!(q.iter_removed().count(), 0);
  assert_eq!(q.iter_update_or_insert().count(), 0);
}

#[derive(Clone)]
struct MapChangesKey<T, F> {
  base: T,
  mapper: F,
}
impl<T, K, F> DataChanges for MapChangesKey<T, F>
where
  T: DataChanges,
  K: CKey,
  F: Fn(T::Key) -> K + Clone + Send + Sync,
{
  type Key = K;
  type Value = T::Value;

  fn has_change(&self) -> bool {
    self.base.has_change()
  }

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self.base.iter_removed().map(self.mapper.clone())
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (K, Self::Value)> + '_ {
    self
      .base
      .iter_update_or_insert()
      .map(|(k, v)| ((self.mapper)(k), v))
  }
}

#[derive(Clone)]
struct DataChangesMap<T, F> {
  base: T,
  mapper: F,
}
impl<T, V, F> DataChanges for DataChangesMap<T, F>
where
  T: DataChanges,
  V: CValue,
  F: Fn(T::Value) -> V + Clone + Send + Sync,
{
  type Key = T::Key;
  type Value = V;

  fn has_change(&self) -> bool {
    self.base.has_change()
  }

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self.base.iter_removed()
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self
      .base
      .iter_update_or_insert()
      .map(|(k, v)| (k, (self.mapper)(v)))
  }
}

#[derive(Clone)]
struct DataChangesFilterMap<T, F> {
  base: T,
  mapper: F,
}
impl<T, V, F> DataChanges for DataChangesFilterMap<T, F>
where
  T: DataChanges,
  F: Fn(T::Value) -> Option<V> + Clone + Send + Sync,
{
  type Key = T::Key;
  type Value = V;

  fn has_change(&self) -> bool {
    self.base.has_change()
  }

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self.base.iter_removed()
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self
      .base
      .iter_update_or_insert()
      .filter_map(|(k, v)| (self.mapper)(v).map(|v2| (k, v2)))
  }
}

#[derive(Clone)]
struct DataChangesFilterKVMap<T, F> {
  base: T,
  mapper: F,
}
impl<T, V, F> DataChanges for DataChangesFilterKVMap<T, F>
where
  T: DataChanges,
  F: Fn(&T::Key, T::Value) -> Option<V> + Clone + Send + Sync,
{
  type Key = T::Key;
  type Value = V;

  fn has_change(&self) -> bool {
    self.base.has_change()
  }

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self.base.iter_removed()
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self
      .base
      .iter_update_or_insert()
      .filter_map(|(k, v)| (self.mapper)(&k, v).map(|v2| (k, v2)))
  }
}

#[test]
fn test_data_changes_map_key() {
  let changes = LinearBatchChanges {
    removed: vec![1u32],
    update_or_insert: vec![(2u32, "a".to_string())],
  };

  let mapped = changes.map_changes_key(|k: u32| k + 100);

  let removed: Vec<_> = mapped.iter_removed().collect();
  assert_eq!(removed, vec![101]);

  let updated: Vec<_> = mapped.iter_update_or_insert().collect();
  assert_eq!(updated, vec![(102, "a".to_string())]);
}

#[test]
fn test_data_changes_collective_map() {
  let changes = LinearBatchChanges {
    removed: vec![1u32],
    update_or_insert: vec![(2u32, "hello".to_string())],
  };

  let mapped = changes.collective_map(|v: String| v.len());

  assert_eq!(mapped.iter_removed().count(), 1);
  let updated: Vec<_> = mapped.iter_update_or_insert().collect();
  assert_eq!(updated, vec![(2u32, 5usize)]);
}

#[test]
fn test_data_changes_collective_filter_map() {
  let changes = LinearBatchChanges {
    removed: vec![1u32],
    update_or_insert: vec![(2u32, 10i32), (3, 25), (4, 5)],
  };

  let filtered = changes.collective_filter_map(|v: i32| if v >= 10 { Some(v * 2) } else { None });

  let updated: Vec<_> = filtered.iter_update_or_insert().collect();
  assert_eq!(updated, vec![(2u32, 20), (3, 50)]);
}

#[test]
fn test_data_changes_collective_filter_kv_map() {
  let changes = LinearBatchChanges {
    removed: vec![1u32],
    update_or_insert: vec![(2u32, 10i32), (3, 20)],
  };

  let filtered =
    changes.collective_filter_kv_map(|k: &u32, v: i32| if *k == 2 { Some(v * 100) } else { None });

  let updated: Vec<_> = filtered.iter_update_or_insert().collect();
  assert_eq!(updated, vec![(2u32, 1000)]);
}

impl<T: DataChanges> DataChanges for Arc<T> {
  type Key = T::Key;
  type Value = T::Value;

  fn has_change(&self) -> bool {
    (**self).has_change()
  }

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    (**self).iter_removed()
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    (**self).iter_update_or_insert()
  }
}

#[derive(Clone, Debug)]
pub struct LinearBatchChanges<K, T> {
  pub removed: Vec<K>,
  pub update_or_insert: Vec<(K, T)>,
}

impl<K, T> LinearBatchChanges<K, T> {
  pub fn reserve(&mut self, size: usize) {
    self.removed.reserve(size);
    self.update_or_insert.reserve(size);
  }

  pub fn is_empty(&self) -> bool {
    self.removed.is_empty() && self.update_or_insert.is_empty()
  }
}

impl<K, T> Default for LinearBatchChanges<K, T> {
  fn default() -> Self {
    Self {
      removed: Default::default(),
      update_or_insert: Default::default(),
    }
  }
}

impl<K: CKey, T: Clone + Send + Sync> DataChanges for LinearBatchChanges<K, T> {
  type Key = K;
  type Value = T;

  fn has_change(&self) -> bool {
    !(self.removed.is_empty() && self.update_or_insert.is_empty())
  }

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self.removed.iter().cloned()
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self.update_or_insert.iter().cloned()
  }
}

#[test]
fn test_linear_batch_changes_basic() {
  let changes = LinearBatchChanges {
    removed: vec![1u32],
    update_or_insert: vec![(2u32, "a".to_string()), (3, "b".to_string())],
  };

  assert!(changes.has_change());

  let removed: Vec<_> = changes.iter_removed().collect();
  assert_eq!(removed, vec![1]);

  let updated: FastHashMap<_, _> = changes.iter_update_or_insert().collect();
  assert_eq!(updated.len(), 2);
  assert_eq!(updated[&2], "a");
  assert_eq!(updated[&3], "b");
}

#[test]
fn test_linear_batch_changes_empty() {
  let changes: LinearBatchChanges<u32, String> = LinearBatchChanges::default();

  assert!(!changes.has_change());
  assert!(changes.is_empty());
  assert_eq!(changes.iter_removed().count(), 0);
  assert_eq!(changes.iter_update_or_insert().count(), 0);
}

pub trait IteratorProvider {
  type Item;
  fn create_iter(&self) -> impl Iterator<Item = &Self::Item> + '_;
}

impl<T, const N: usize> IteratorProvider for [T; N] {
  type Item = T;

  fn create_iter(&self) -> impl Iterator<Item = &Self::Item> + '_ {
    self.iter()
  }
}
impl<T> IteratorProvider for Vec<T> {
  type Item = T;

  fn create_iter(&self) -> impl Iterator<Item = &Self::Item> + '_ {
    self.iter()
  }
}

#[derive(Clone)]
pub struct SelectChanges<T>(pub T);

impl<T> DataChanges for SelectChanges<T>
where
  T: IteratorProvider + Clone + Send + Sync,
  T::Item: DataChanges,
{
  type Key = <T::Item as DataChanges>::Key;
  type Value = <T::Item as DataChanges>::Value;

  fn has_change(&self) -> bool {
    self.0.clone().create_iter().all(|c| c.has_change())
  }

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self.0.create_iter().flat_map(|c| c.iter_removed())
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self.0.create_iter().flat_map(|c| c.iter_update_or_insert())
  }
}

#[test]
fn test_select_changes_basic() {
  let c1 = LinearBatchChanges {
    removed: vec![1u32],
    update_or_insert: vec![(2u32, "a".to_string())],
  };
  let c2 = LinearBatchChanges {
    removed: vec![3u32],
    update_or_insert: vec![(4u32, "b".to_string())],
  };

  let selects = SelectChanges(vec![c1, c2]);

  let removed: FastHashSet<_> = selects.iter_removed().collect();
  assert_eq!(removed.len(), 2);
  assert!(removed.contains(&1));
  assert!(removed.contains(&3));

  let updated: FastHashMap<_, _> = selects.iter_update_or_insert().collect();
  assert_eq!(updated.len(), 2);
  assert_eq!(updated[&2], "a");
  assert_eq!(updated[&4], "b");
}

pub fn merge_linear_batch_changes<K: CKey, T: Clone>(
  changes: &mut Vec<Arc<LinearBatchChanges<K, T>>>,
) -> Arc<LinearBatchChanges<K, T>> {
  if changes.len() == 1 {
    return changes.pop().unwrap();
  }

  let mut removes = FastHashSet::default();
  let mut new_inserts = FastHashMap::default();

  // the drain iter has correct visit order
  for change in changes.drain(..) {
    for k in change.removed.iter() {
      removes.insert(k.clone());

      new_inserts.remove(k);
    }
    for (k, v) in change.update_or_insert.iter() {
      new_inserts.insert(k.clone(), v.clone());
      removes.remove(k);
    }
  }

  Arc::new(LinearBatchChanges {
    removed: removes.into_iter().collect(),
    update_or_insert: new_inserts.into_iter().collect(),
  })
}

#[test]
fn test_merge_linear_batch_changes_single() {
  let c1 = Arc::new(LinearBatchChanges {
    removed: vec![1u32],
    update_or_insert: vec![(2u32, "a".to_string())],
  });
  let mut changes = vec![c1.clone()];

  let merged = merge_linear_batch_changes(&mut changes);
  assert!(changes.is_empty());

  let removed: Vec<_> = merged.iter_removed().collect();
  assert_eq!(removed, vec![1]);
  let updated: Vec<_> = merged.iter_update_or_insert().collect();
  assert_eq!(updated, vec![(2, "a".to_string())]);
}

#[test]
fn test_merge_linear_batch_changes_remove_wins() {
  // key 2 is updated in c1, then removed in c2 → should be removed only
  let c1 = Arc::new(LinearBatchChanges {
    removed: vec![1u32],
    update_or_insert: vec![(2u32, "a".to_string())],
  });
  let c2 = Arc::new(LinearBatchChanges {
    removed: vec![2u32],
    update_or_insert: vec![],
  });
  let mut changes = vec![c1, c2];

  let merged = merge_linear_batch_changes(&mut changes);

  let removed: Vec<_> = merged.iter_removed().collect();
  assert_eq!(removed.len(), 2);
  assert!(removed.contains(&1));
  assert!(removed.contains(&2));

  assert_eq!(merged.iter_update_or_insert().count(), 0);
}

#[test]
fn test_merge_linear_batch_changes_insert_wins() {
  // key 2 is removed in c1, then re-inserted in c2 → should be inserted
  let c1 = Arc::new(LinearBatchChanges {
    removed: vec![2u32],
    update_or_insert: vec![],
  });
  let c2 = Arc::new(LinearBatchChanges {
    removed: vec![],
    update_or_insert: vec![(2u32, "new".to_string())],
  });
  let mut changes = vec![c1, c2];

  let merged = merge_linear_batch_changes(&mut changes);

  assert_eq!(merged.iter_removed().count(), 0);
  let updated: Vec<_> = merged.iter_update_or_insert().collect();
  assert_eq!(updated, vec![(2, "new".to_string())]);
}
