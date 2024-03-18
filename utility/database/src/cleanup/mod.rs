use crate::*;

struct MetaEntityTable;
// declare_component!(MetaEntityTypeId, MetaEntityTable, TypeId);
declare_component!(MetaEntityTypeName, MetaEntityTable, String);

struct MetaComponentTable;
// declare_component!(MetaComponentTypeId, MetaComponentTable, TypeId);
declare_component!(MetaComponentTypeName, MetaEntityTable, String);
// declare foreign key

pub trait DatabaseRAIIExt {
  fn enable_raii(&self);
  fn cleanup_zero_referenced_entities(&self);
}

impl DatabaseRAIIExt for Database {
  fn enable_raii(&self) {
    self.entity_meta_watcher.on(|entity| {
      // todo inject refcount_table
      // todo add ref count maintain logic

      false
    });
  }

  fn cleanup_zero_referenced_entities(&self) {
    //
  }
}

pub struct DataRAIIPtr<T> {
  inner: EntityHandle<T>,
  ref_count_storage: Arc<RwLock<Vec<u32>>>,
}

impl<T> Drop for DataRAIIPtr<T> {
  fn drop(&mut self) {
    let refs = self.ref_count_storage.write();
    let ref_count = &mut refs[self.inner.alloc_idx().index as usize];
    assert!(*ref_count >= 1);
    *ref_count -= 1;
  }
}
