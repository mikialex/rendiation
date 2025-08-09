use crate::*;

/// abstract batch change container
/// - removing and mutation is separated, because removing likely to be consumed first
/// - not care about previous value
pub trait DataChanges: Send + Sync + Clone {
  type Key: CKey;
  type Value: CValue;
  fn has_change(&self) -> bool;
  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_;
  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_;

  fn materialize(self) -> Arc<LinearBatchChanges<Self::Key, Self::Value>> {
    Arc::new(LinearBatchChanges {
      removed: self.iter_removed().collect(),
      update_or_insert: self.iter_update_or_insert().collect(),
    })
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

#[derive(Clone)]
pub struct LinearBatchChanges<K, T> {
  pub removed: Vec<K>,
  pub update_or_insert: Vec<(K, T)>,
}

impl<K, T> Default for LinearBatchChanges<K, T> {
  fn default() -> Self {
    Self {
      removed: Default::default(),
      update_or_insert: Default::default(),
    }
  }
}

impl<K: CKey, T: CValue> DataChanges for LinearBatchChanges<K, T> {
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

#[allow(dead_code)]
const DEBUG_CHECK: bool = true;

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
    if DEBUG_CHECK {
      let mut keys_check = FastHashSet::default();
      for s in self.0.create_iter() {
        for r in s.iter_removed() {
          assert!(keys_check.insert(r))
        }
      }
    }
    self.0.create_iter().flat_map(|c| c.iter_removed())
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    if DEBUG_CHECK {
      let mut keys_check = FastHashSet::default();
      keys_check.clear();
      for s in self.0.create_iter() {
        for (r, _) in s.iter_update_or_insert() {
          assert!(keys_check.insert(r))
        }
      }
    }
    self.0.create_iter().flat_map(|c| c.iter_update_or_insert())
  }
}
