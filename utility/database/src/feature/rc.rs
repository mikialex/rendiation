use std::pin::Pin;
use std::task::{Context, Poll};

use futures::task::{noop_waker, AtomicWaker};
use futures::Stream;

use crate::*;

#[derive(Clone)]
pub struct DatabaseEntityRefCounting {
  entity_ref_counts: Arc<RwLock<StreamMap<TypeId, EntityRefCount>>>,
  cleanup_waker: Arc<AtomicWaker>,
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
    let waker = self.cleanup_waker.take().unwrap_or(noop_waker());
    self.cleanup_waker.register(&waker);
    let mut cx = Context::from_waker(&waker);
    let mut entity_ref_counts = self.entity_ref_counts.write();
    entity_ref_counts.poll_until_pending_not_care_result(&mut cx);
  }

  pub fn data_ref_ptr_creator<E: 'static>(&self) -> DataRefPtrCreator<E> {
    let entity_ref_counts = self.entity_ref_counts.read();
    let entity_ref_count = entity_ref_counts.get(&TypeId::of::<E>()).unwrap();
    let entity_checker = entity_ref_count
      .outer_refs
      .read()
      .entity_checker
      .make_read_holder();
    DataRefPtrCreator {
      phantom_data: PhantomData,
      ref_data: entity_ref_count.outer_refs.make_write_holder(),
      entity_checker,
    }
  }
}

pub struct EntityRefCount {
  outer_refs: Arc<RwLock<ExternalDataRefs>>,
  inner_refs: CollectionSetsRefcount<u32>,
  db: Database,
  id: TypeId,
}

impl Stream for EntityRefCount {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    if let Poll::Ready(change) = self.inner_refs.poll_changes(cx) {
      let outer_count = self.outer_refs.read();

      for (k, change) in change.iter_key_value() {
        if change.is_removed() && outer_count.ref_counts[k as usize] != 0 {
          todo!()
        }
      }
    }

    Poll::Pending
  }
}

struct ExternalDataRefs {
  entity_checker: Arc<RwLock<Arena<()>>>,
  ref_counts: Vec<u32>,
  waker: AtomicWaker,
}

pub struct DataRefPtrCreator<T> {
  phantom_data: PhantomData<T>,
  ref_data: LockWriteGuardHolder<ExternalDataRefs>,
  entity_checker: LockReadGuardHolder<Arena<()>>,
}

impl<T> DataRefPtrCreator<T> {
  pub fn create_ptr(&mut self, handle: EntityHandle<T>) -> DataRefPtr<T> {
    if !self.entity_checker.contains(handle.handle) {
      panic!("handle is invalid")
    }
    // todo inc refcount
    DataRefPtr {
      inner: handle,
      ref_count_storage: self.ref_data.get_lock(),
    }
  }
}

impl<T> Drop for DataRefPtrCreator<T> {
  fn drop(&mut self) {
    self.ref_data.waker.wake();
  }
}

/// User could keep these ptr outside of the database to keep the entity alive;
/// The foreign relations between different entities will be automatically considered.
pub struct DataRefPtr<T> {
  inner: EntityHandle<T>,
  ref_count_storage: Arc<RwLock<ExternalDataRefs>>,
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
    let ref_count = &mut refs.ref_counts[self.inner.alloc_idx().index as usize];
    assert!(*ref_count >= 1);
    *ref_count -= 1;
    refs.waker.wake();
  }
}

impl<T> Clone for DataRefPtr<T> {
  fn clone(&self) -> Self {
    let mut refs = self.ref_count_storage.write();
    let ref_count = &mut refs.ref_counts[self.inner.alloc_idx().index as usize];
    *ref_count += 1;

    Self {
      inner: self.inner,
      ref_count_storage: self.ref_count_storage.clone(),
    }
  }
}
