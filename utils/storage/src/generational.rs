use crate::*;

use arena::{Arena, Handle as ArenaHandle};

pub struct GenerationalVecStorage;

impl<T> StorageBehavior<T> for GenerationalVecStorage {
  type Container = Arena<T>;
  type Handle = ArenaHandle<T>;

  fn insert(c: &mut Self::Container, v: T) -> Handle<T, Self> {
    Handle::new(c.insert(v))
  }
  fn remove(c: &mut Self::Container, handle: Self::Handle) -> Option<T> {
    c.remove(handle)
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
