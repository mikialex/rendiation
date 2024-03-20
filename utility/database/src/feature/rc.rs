use std::pin::Pin;
use std::task::{Context, Poll};

use futures::task::AtomicWaker;
use futures::Stream;

use crate::*;

pub struct DatabaseEntityRefCounting {
  entity_ref_counts: FastHashMap<TypeId, EntityRefCount>,
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

  pub fn data_ref_ptr_creator<C: ComponentSemantic>(&self) -> DataRefPtrCreator<C::Entity> {
    todo!()
  }
}

pub struct EntityRefCount {
  db_outer_ref_count: Arc<RwLock<ExternalDataRefs>>,
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

struct ExternalDataRefs {
  ref_counts: Vec<u32>,
  waker: AtomicWaker,
}

pub struct DataRefPtrCreator<T> {
  phantom_data: PhantomData<T>,
  ref_data: LockWriteGuardHolder<ExternalDataRefs>,
}

impl<T> DataRefPtrCreator<T> {
  pub fn create_ptr(&mut self, handle: EntityHandle<T>) -> DataRefPtr<T> {
    // todo check handle is valid
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
