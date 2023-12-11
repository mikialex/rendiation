use rayon::iter::plumbing::{bridge_unindexed, Folder, UnindexedConsumer, UnindexedProducer};

use crate::*;

pub trait VirtualTable<K, V>: Send + Sync {
  /// O(1) cost
  fn get_value(&self, key: &K) -> Option<V>;
  fn contains(&self, key: &K) -> bool {
    self.get_value(key).is_some()
  }
  fn iter(&self) -> Box<dyn Iterator<Item = (K, V)> + '_>;
  fn split_table(&self) -> Option<(Box<dyn VirtualTable<K, V>>, Box<dyn VirtualTable<K, V>>)>;
}

impl<K: Send + Sync, V: Send + Sync> ParallelIterator for Box<dyn VirtualTable<K, V>> {
  type Item = (K, V);

  fn drive_unindexed<C>(self, consumer: C) -> C::Result
  where
    C: UnindexedConsumer<Self::Item>,
  {
    bridge_unindexed(self, consumer)
  }
}

impl<K, V> UnindexedProducer for Box<dyn VirtualTable<K, V>> {
  type Item = (K, V);

  fn split(self) -> (Self, Option<Self>) {
    if let Some((left, right)) = self.split_table() {
      (left, Some(right))
    } else {
      (self, None)
    }
  }

  fn fold_with<F>(self, folder: F) -> F
  where
    F: Folder<Self::Item>,
  {
    folder.consume_iter(self.iter())
  }
}

pub enum TableValueChange<T> {
  Removed(T),
  Changed(T, Option<T>),
}

impl<T> TableValueChange<T> {
  pub fn previous(&self) -> Option<&T> {
    match self {
      TableValueChange::Removed(v) => Some(v),
      TableValueChange::Changed(_, v) => v.as_ref(),
    }
  }
  pub fn into_previous(self) -> Option<T> {
    match self {
      TableValueChange::Removed(v) => Some(v),
      TableValueChange::Changed(_, v) => v,
    }
  }
}

pub trait IncrementalTable<K, V> {
  fn poll_changes(&mut self, cx: &mut Context) -> CPoll<TableTransaction<K, V>>;
}

pub struct TableTransaction<'a, K, V> {
  pub current: Box<dyn VirtualTable<K, V> + 'a>,
  pub delta: Box<dyn VirtualTable<K, TableValueChange<V>> + 'a>,
}

pub struct TablePreviousView<'a, 'b, K, V> {
  change_and_source: &'b TableTransaction<'a, K, V>,
}

/// the impl access the previous V
impl<'a, 'b, K, V: Clone> VirtualTable<K, V> for TablePreviousView<'a, 'b, K, V> {
  fn get_value(&self, key: &K) -> Option<V> {
    let info = &self.change_and_source;
    if let Some(change) = info.delta.get_value(key) {
      change.previous().cloned()
    } else {
      info.current.get_value(key)
    }
  }

  fn iter(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
    let info = &self.change_and_source;
    let current_not_changed = info.current.iter().filter(|(k, _)| !info.delta.contains(k));

    let current_changed = info
      .delta
      .iter()
      .filter_map(|(k, v)| v.into_previous().map(|v| (k, v)));

    Box::new(current_not_changed.chain(current_changed))
  }

  fn split_table(&self) -> Option<(Box<dyn VirtualTable<K, V>>, Box<dyn VirtualTable<K, V>>)> {
    // self.change_and_source.split();
    todo!()
  }
}

// add pre to all message
// merge access and poll changes lock acquire
// abstract over delta table using the same abstraction for original table and delta table
// not loop from the root
