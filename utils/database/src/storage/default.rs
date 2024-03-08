use crate::*;

impl<T: CValue + Default> ComponentStorage<T> for Arc<RwLock<Vec<T>>> {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView<T>> {
    Box::new(self.make_read_holder())
  }

  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView<T>> {
    Box::new(self.make_write_holder())
  }
}

impl<T> ComponentStorageReadView<T> for LockReadGuardHolder<Vec<T>> {
  fn get(&self, idx: usize) -> Option<&T> {
    self.deref().get(idx)
  }
}
impl<T> ComponentStorageReadView<T> for LockWriteGuardHolder<Vec<T>> {
  fn get(&self, idx: usize) -> Option<&T> {
    self.deref().get(idx)
  }
}
impl<T: Clone + Default> ComponentStorageReadWriteView<T> for LockWriteGuardHolder<Vec<T>> {
  fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
    let data: &mut Vec<T> = self;
    if data.len() <= idx {
      data.resize(idx + 1, T::default());
    }
    data.get_mut(idx)
  }
}
