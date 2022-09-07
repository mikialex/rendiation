use crate::*;

pub use arena::{Arena, Handle as ArenaHandle};

impl<T> StorageBehavior<T> for Arena<T> {
  type Handle = ArenaHandle<T>;

  fn insert(&mut self, v: T) -> Handle<T, Self> {
    Handle::new(self.insert(v))
  }
  fn remove(&mut self, handle: Self::Handle) -> Option<T> {
    self.remove(handle)
  }
  fn get(&self, handle: Self::Handle) -> Option<&T> {
    self.get(handle)
  }
  fn get_mut(&mut self, handle: Self::Handle) -> Option<&mut T> {
    self.get_mut(handle)
  }
  fn size(&self) -> usize {
    self.len()
  }
}

impl<T> NoneOverlappingStorage<T> for Arena<T> {
  fn get_mut_pair(&mut self, handle: (Self::Handle, Self::Handle)) -> Option<(&mut T, &mut T)> {
    let (a, b) = self.get2_mut(handle.0, handle.1);
    (a?, b?).into()
  }
}

impl<T> HandlePredictableStorage<T> for Arena<T> {
  fn insert_with(&mut self, creator: impl FnOnce(Handle<T, Self>) -> T) -> Handle<T, Self> {
    Handle::new(self.insert_with(|handle| creator(Handle::new(handle))))
  }
}
