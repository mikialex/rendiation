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

  fn collective_filter_map<V: CValue>(
    self,
    f: impl Fn(Self::Value) -> Option<V> + Clone + Send + Sync + 'static,
  ) -> impl DataChanges<Key = Self::Key, Value = V> {
    DataChangesFilterMap {
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
  V: CValue,
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

pub fn merge_linear_batch_changes<K: CKey, T: Clone>(
  changes: &[Arc<LinearBatchChanges<K, T>>],
) -> LinearBatchChanges<K, T> {
  let mut removes = FastHashSet::default();
  let mut new_inserts = FastHashMap::default();

  for change in changes {
    for k in change.removed.iter() {
      removes.insert(k.clone());

      new_inserts.remove(k);
    }
    for (k, v) in change.update_or_insert.iter() {
      new_inserts.insert(k.clone(), v.clone());
      removes.remove(k);
    }
  }

  LinearBatchChanges {
    removed: removes.into_iter().collect(),
    update_or_insert: new_inserts.into_iter().collect(),
  }
}
