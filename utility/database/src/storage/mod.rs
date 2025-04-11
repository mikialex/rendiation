use crate::*;
mod default;

pub type DataPtr = *const ();
pub type DataMutPtr = *const ();

pub trait ComponentStorage: Send + Sync + DynClone {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView>;
  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView>;
}
dyn_clone::clone_trait_object!(ComponentStorage);

pub trait ComponentStorageReadView: Send + Sync + DynClone {
  fn get(&self, idx: u32) -> Option<DataPtr>;
  /// # Safety
  ///
  /// caller must ensure the idx is inbound
  unsafe fn get_unchecked(&self, idx: u32) -> DataPtr {
    self.get(idx).unwrap_unchecked()
  }
  fn read_component_into_boxed(&self, idx: u32) -> Option<Box<dyn Any>>;
}
dyn_clone::clone_trait_object!(ComponentStorageReadView);

pub trait ComponentStorageReadWriteView {
  fn get(&self, idx: u32) -> Option<DataPtr>;
  fn get_mut(&mut self, idx: u32) -> Option<DataMutPtr>;
  /// # Safety
  ///
  /// this method should not called by user, but should only called in entity writer
  /// because only it will ensure the all components write lock is held, which is required in
  /// interleaved storage implementation
  unsafe fn grow_at_least(&mut self, max: usize);
}
