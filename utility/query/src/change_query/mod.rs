use crate::*;

/// abstract batch change container
/// - removing and mutation is separated, because removing likely be consumed first
/// - not care about previous value
pub trait DataChanges: Send + Sync + Clone {
  type Key: CKey;
  type Value: CValue;
  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_;
  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_;

  fn collective_map<V: CValue>(
    self,
    f: impl Fn(Self::Value) -> V + Clone + Send + Sync + 'static,
  ) -> impl DataChanges<Key = Self::Key, Value = V> {
    DataChangesMap {
      base: self,
      mapper: f,
    }
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

impl<T: DataChanges> DataChanges for Arc<T> {
  type Key = T::Key;
  type Value = T::Value;

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    (**self).iter_removed()
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    (**self).iter_update_or_insert()
  }
}

/// - update/change is linear, accessed by u32 index
#[derive(Clone)]
pub struct LinearBatchChanges<T> {
  pub removed: Vec<u32>,
  pub update_or_insert: Vec<(u32, T)>,
}

impl<T: CValue> DataChanges for LinearBatchChanges<T> {
  type Key = u32;
  type Value = T;

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self.removed.iter().copied()
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self.update_or_insert.iter().cloned()
  }
}

// pub struct TypedLinearBatchChanges<K, T> {
//   marker: PhantomData<K>,
//   pub internal: LinearBatchChanges<T>,
// }
