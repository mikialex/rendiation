use crate::*;

pub struct VecStorage;

impl<T> StorageBehavior<T> for VecStorage {
  type Container = Vec<T>;
  type Handle = usize;

  fn insert(c: &mut Self::Container, v: T) -> Handle<T, Self> {
    c.push(v);
    Handle::new(c.len() - 1)
  }
  fn get(c: &Self::Container, handle: Self::Handle) -> Option<&T> {
    c.get(handle)
  }
  fn get_mut(c: &mut Self::Container, handle: Self::Handle) -> Option<&mut T> {
    c.get_mut(handle)
  }
  fn size(c: &Self::Container) -> usize {
    c.len()
  }
}

pub struct DeduplicateVecStorage;
impl<T: PartialEq + Copy> StorageBehavior<T> for DeduplicateVecStorage {
  type Container = Vec<T>;
  type Handle = usize;

  fn insert(c: &mut Self::Container, v: T) -> Handle<T, Self> {
    c.push(v);
    let index = c.iter().position(|&cv| cv == v).unwrap_or_else(|| {
      c.push(v);
      c.len() - 1
    });
    Handle::new(index)
  }

  fn get(c: &Self::Container, handle: Self::Handle) -> Option<&T> {
    c.get(handle)
  }
  fn get_mut(c: &mut Self::Container, handle: Self::Handle) -> Option<&mut T> {
    c.get_mut(handle)
  }
  fn size(c: &Self::Container) -> usize {
    c.len()
  }
}
