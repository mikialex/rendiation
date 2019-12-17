use core::hash::Hasher;
use core::hash::Hash;

pub struct BufferData<T> {
  pub id: usize,
  pub data: Vec<T>,
  pub stride: usize,
}

impl<T> Hash for BufferData<T> {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.id.hash(state);
  }
}

impl<T> PartialEq for BufferData<T> {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}
impl<T> Eq for BufferData<T> {}

impl<T> BufferData<T> {
  pub fn new(id: usize, data: Vec<T>, stride: usize) -> BufferData<T> {
    BufferData { id, data, stride }
  }
}
