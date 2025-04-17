use crate::*;
mod default;

pub use default::*;

pub type DataPtr = *const ();
pub type DataMutPtr = *const ();

/// This trait encapsulate the implementation of component storage.
/// For different kinds of component, we can have different storage implementation.
/// For example. If the component data is sparse, we could using hashmap as the storage
/// to improve the space efficiency at the cost of access performance. If the multiple
/// component data is exclusively exists, we can use a enum like buffer to improve the
/// space efficiency. If the multiple component will always accessed together, we could
/// store them in a interleaved buffer like common AOS way to improve the access performance.
pub trait ComponentStorage: Send + Sync + DynClone {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView>;
  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView>;
  fn type_id(&self) -> TypeId;
  fn data_shape(&self) -> &'static facet::Shape;
}
dyn_clone::clone_trait_object!(ComponentStorage);

pub trait ComponentStorageReadView: Send + Sync + DynClone {
  /// get the data located in idx, return None if out of bound.
  fn get(&self, idx: u32) -> Option<DataPtr>;
  /// this function will be removed in future.
  fn debug_value(&self, idx: u32) -> Option<String>;
}
dyn_clone::clone_trait_object!(ComponentStorageReadView);

pub trait ComponentStorageReadWriteView {
  /// get the data located in idx.
  fn get(&self, idx: u32) -> Option<DataPtr>;
  /// this function will be removed in future.
  fn debug_value(&self, idx: u32) -> Option<String>;

  /// return if success
  ///
  /// the idx is handle, but only used for emit message, generation check
  /// should have been done outside
  ///
  /// The index must in bounded with the max grow size. [ComponentStorageReadWriteView::grow]
  fn set_value(
    &mut self,
    idx: RawEntityHandle,
    v: DataPtr,
    is_create: bool,
    event: &mut Source<ChangePtr>,
  ) -> bool;

  /// return if success
  ///
  /// the idx is handle, but only used for emit message, generation check
  /// should have been done outside.
  ///
  /// The implementation should emit the proper event on event dispatcher
  fn set_default_value(
    &mut self,
    idx: RawEntityHandle,
    is_create: bool,
    event: &mut Source<ChangePtr>,
  ) -> bool;

  /// the idx is handle, but only used for emit message, generation check
  /// should have been done outside
  ///
  /// The implementation should emit the proper event on event dispatcher
  fn delete(&mut self, idx: RawEntityHandle, event: &mut Source<ChangePtr>);

  /// The implementation should emit the proper event on event dispatcher
  fn notify_start_mutation(&mut self, event: &mut Source<ChangePtr>);
  /// The implementation should emit the proper event on event dispatcher
  fn notify_end_mutation(&mut self, event: &mut Source<ChangePtr>);

  /// grow the storage to allow more data to stored below the max size address.
  fn grow(&mut self, max: u32);
}
