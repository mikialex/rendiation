use crate::*;

impl<T: CValue + Default> ComponentStorage<T> for Arc<RwLock<Vec<T>>> {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView<T>> {
    Box::new(self.make_read_holder())
  }

  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView<T>> {
    Box::new(self.make_write_holder())
  }
}

impl<T: CValue> ComponentStorageReadView<T> for LockReadGuardHolder<Vec<T>> {
  fn get(&self, idx: usize) -> Option<&T> {
    self.deref().get(idx)
  }

  fn clone_read_view(&self) -> Box<dyn ComponentStorageReadView<T>> {
    Box::new(self.clone())
  }
}

impl<T: CValue + Default> ComponentStorageReadWriteView<T> for LockWriteGuardHolder<Vec<T>> {
  fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
    let data: &mut Vec<T> = self;
    data.get_mut(idx)
  }
  fn get(&self, idx: usize) -> Option<&T> {
    let data: &Vec<T> = self;
    data.get(idx)
  }

  unsafe fn grow_at_least(&mut self, max: usize) {
    let data: &mut Vec<T> = self;
    if data.len() <= max {
      data.resize(max + 1, T::default());
    }
  }
}
