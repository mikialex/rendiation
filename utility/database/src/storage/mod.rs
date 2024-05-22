mod interleave;
pub use interleave::*;
mod default;

pub trait ComponentStorage<T>: Send + Sync {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView<T>>;
  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView<T>>;
}

pub trait ComponentStorageReadView<T>: Send + Sync {
  fn get(&self, idx: usize) -> Option<&T>;
  fn clone_read_view(&self) -> Box<dyn ComponentStorageReadView<T>>;
}
pub trait ComponentStorageReadWriteView<T> {
  /// # Safety
  ///
  /// this method should not called by user, but should only called in entity writer
  /// because only it will ensure the all components write lock is held, which is required in
  /// interleaved storage implementation
  unsafe fn grow_at_least(&mut self, max: usize);
  fn get(&self, idx: usize) -> Option<&T>;
  fn get_mut(&mut self, idx: usize) -> Option<&mut T>;
}
