mod interleave;
pub use interleave::*;
mod default;

pub trait ComponentStorage<T>: Send + Sync {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView<T>>;
  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView<T>>;
}

pub trait ComponentStorageReadView<T> {
  fn get(&self, idx: usize) -> Option<&T>;
}
pub trait ComponentStorageReadWriteView<T>: ComponentStorageReadView<T> {
  fn grow_at_least(&mut self, max: usize);
  fn get_mut(&mut self, idx: usize) -> Option<&mut T>;
}
