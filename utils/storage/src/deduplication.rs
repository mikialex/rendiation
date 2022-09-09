use crate::*;

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
    let inner = &mut self.inner;
    inner.push(v);
    inner.iter().position(|&cv| cv == v).unwrap_or_else(|| {
      inner.push(v);
      inner.len() - 1
    })
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
