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
  fn get(&self, idx: RawEntityHandle) -> Option<&T> {
    // todo generation check
    self.deref().get(idx.index() as usize)
  }
  fn get_without_generation_check(&self, idx: u32) -> Option<&T> {
    self.deref().get(idx as usize)
  }

  fn clone_read_view(&self) -> Box<dyn ComponentStorageReadView<T>> {
    Box::new(self.clone())
  }
}

impl<T: CValue + Default> ComponentStorageReadWriteView<T> for LockWriteGuardHolder<Vec<T>> {
  fn get_mut(&mut self, idx: RawEntityHandle) -> Option<&mut T> {
    // todo generation check
    let data: &mut Vec<T> = self;
    data.get_mut(idx.index() as usize)
  }
  fn get(&self, idx: RawEntityHandle) -> Option<&T> {
    // todo generation check
    let data: &Vec<T> = self;
    data.get(idx.index() as usize)
  }

  unsafe fn grow_at_least(&mut self, max: usize) {
    let data: &mut Vec<T> = self;
    if data.len() <= max {
      data.resize(max + 1, T::default());
    }
  }
}
