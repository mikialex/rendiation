mod interleave;
pub use interleave::*;
mod default;

use crate::*;

pub trait ComponentStorage<T>: Send + Sync {
  fn create_read_view(&self) -> Arc<dyn ComponentStorageReadView<T>>;
  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView<T>>;
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait ComponentStorageReadView<T>: Send + Sync {
  fn get(&self, idx: usize) -> Option<&T>;
}
pub trait ComponentStorageReadWriteView<T>: ComponentStorageReadView<T> {
  /// # Safety
  ///
  /// this method should not called by user, but should only called in entity writer
  /// because only it will ensure the all components write lock is held, which is required in
  /// interleaved storage implementation
  unsafe fn grow_at_least(&mut self, max: usize);
  fn get_mut(&mut self, idx: usize) -> Option<&mut T>;
}
