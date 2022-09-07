use crate::*;

pub struct VecStorage;

impl<T> StorageBehavior<T> for Vec<T> {
  type Handle = usize;

  fn insert(&mut self, v: T) -> Self::Handle {
    self.push(v);
    self.len() - 1
  }
  fn remove(&mut self, handle: Self::Handle) -> Option<T> {
    todo!()
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

pub struct DeduplicateVec<T> {
  inner: Vec<T>,
}

impl<T> Default for DeduplicateVec<T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}

impl<T: PartialEq + Copy> StorageBehavior<T> for DeduplicateVec<T> {
  type Handle = usize;

  fn insert(&mut self, v: T) -> Self::Handle {
    // self.push(v);
    // let index = self.iter().position(|&cv| cv == v).unwrap_or_else(|| {
    //   self.push(v);
    //   self.len() - 1
    // });
    // Handle::new(index)
    todo!()
  }
  fn remove(&mut self, handle: Self::Handle) -> Option<T> {
    todo!()
  }

  fn get(&self, handle: Self::Handle) -> Option<&T> {
    self.inner.get(handle)
  }
  fn get_mut(&mut self, handle: Self::Handle) -> Option<&mut T> {
    self.inner.get_mut(handle)
  }
  fn size(&self) -> usize {
    self.inner.len()
  }
}
