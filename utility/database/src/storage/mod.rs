use crate::*;
mod default;

pub use default::*;

pub type DataPtr = *const ();
pub type DataMutPtr = *const ();

pub trait ComponentStorage: Send + Sync + DynClone {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView>;
  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView>;
}
dyn_clone::clone_trait_object!(ComponentStorage);

pub trait ComponentStorageReadView: Send + Sync + DynClone {
  fn get(&self, idx: u32) -> Option<DataPtr>;
  fn debug_value(&self, idx: u32) -> Option<String>;
  fn type_id(&self) -> TypeId;
}
dyn_clone::clone_trait_object!(ComponentStorageReadView);

pub trait ComponentStorageReadWriteView {
  fn get(&self, idx: u32) -> Option<DataPtr>;

  /// return if success
  ///
  /// the idx is handle, but only used for emit message, should not do generation check
  fn set_value(
    &mut self,
    idx: RawEntityHandle,
    v: DataPtr,
    is_create: bool,
    event: &mut Source<ChangePtr>,
  ) -> bool;

  /// return if success
  ///
  /// the idx is handle, but only used for emit message, should not do generation check
  fn set_default_value(
    &mut self,
    idx: RawEntityHandle,
    is_create: bool,
    event: &mut Source<ChangePtr>,
  ) -> bool;

  /// the idx is handle, but only used for emit message, should not do generation check
  fn delete(&mut self, idx: RawEntityHandle, event: &mut Source<ChangePtr>);

  fn notify_start_mutation(&mut self, event: &mut Source<ChangePtr>);
  fn notify_end_mutation(&mut self, event: &mut Source<ChangePtr>);

  fn grow(&mut self, max: u32);

  fn debug_value(&self, idx: u32) -> Option<String>;
  fn type_id(&self) -> TypeId;
}
