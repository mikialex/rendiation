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
}
dyn_clone::clone_trait_object!(ComponentStorageReadView);

pub trait ComponentStorageReadWriteView {
  fn get(&self, idx: u32) -> Option<DataPtr>;
  fn get_mut(&mut self, idx: u32) -> Option<DataMutPtr>;
  /// return if success
  fn set_value(&mut self, idx: u32, v: DataPtr) -> bool;
  /// return if success
  fn set_default_value(&mut self, idx: u32) -> bool;
  /// # Safety
  ///
  /// This method should not called by user, but should only called in entity
  /// writer when create new entity.
  unsafe fn grow_at_least(&mut self, max: usize);
}
