use std::pin::Pin;
use std::task::{Context, Poll};

use futures::task::AtomicWaker;
use futures::Stream;

use crate::*;

pub struct DatabaseEntityRefCounting {
  entity_ref_counts: FastHashMap<TypeId, CollectionSetsRefcount<u32>>,
  db: Database,
}

impl DatabaseEntityRefCounting {
  pub fn new(db: &Database) -> Self {
    // todo, we should do loop check in dep graph to warning circular dep
    db.entity_meta_watcher.on(|entity| {
      // todo inject refcount_table
      // todo add ref count maintain logic

      false
    });

    todo!()
  }
  pub fn cleanup_none_referenced_entities(&self) {
    //
  }
}

pub struct EntityRefCount {
  db_outer_ref_count: Arc<RwLock<Vec<u32>>>,
  db_inner_ref_count: CollectionSetsRefcount<u32>,
  db: Database,
  id: TypeId,
}

impl Stream for EntityRefCount {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    todo!()
  }
}

#[derive(Clone)]
pub struct CollectionSetsRefcount<T> {
  source_sets: Arc<Vec<Box<dyn ReactiveCollection<T, ()>>>>,
  wake_for_new_source: Arc<AtomicWaker>,
  ref_count: Arc<RwLock<FastHashMap<T, u32>>>,
}

impl<T> CollectionSetsRefcount<T> {
  pub fn add_source(&self, source: Box<dyn ReactiveCollection<T, ()>>) {
    // self.source_sets.
    // todo
  }
}

impl<T: CKey> VirtualCollection<T, u32> for CollectionSetsRefcount<T> {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (T, u32)> + '_> {
    todo!()
  }

  fn access(&self, key: &T) -> Option<u32> {
    todo!()
  }
}

pub struct DataRefPtr<T> {
  inner: EntityHandle<T>,
  ref_count_storage: Arc<RwLock<Vec<u32>>>,
}

impl<T> Deref for DataRefPtr<T> {
  type Target = EntityHandle<T>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T> Drop for DataRefPtr<T> {
  fn drop(&mut self) {
    let mut refs = self.ref_count_storage.write();
    let ref_count = &mut refs[self.inner.alloc_idx().index as usize];
    assert!(*ref_count >= 1);
    *ref_count -= 1;
  }
}
