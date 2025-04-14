use crate::*;

impl<T: CValue + Default> ComponentStorage for Arc<RwLock<Vec<T>>> {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView> {
    Box::new(self.make_read_holder())
  }

  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView> {
    Box::new(self.make_write_holder())
  }
}

impl<T: CValue> ComponentStorageReadView for LockReadGuardHolder<Vec<T>> {
  fn get(&self, idx: u32) -> Option<DataPtr> {
    self
      .deref()
      .get(idx as usize)
      .map(|r| r as *const _ as DataPtr)
  }
}

impl<T: CValue + Default> ComponentStorageReadWriteView for LockWriteGuardHolder<Vec<T>> {
  fn get_mut(&mut self, idx: u32) -> Option<DataMutPtr> {
    let data: &mut Vec<T> = self;
    data
      .get_mut(idx as usize)
      .map(|r| r as *const _ as DataMutPtr)
  }

  fn get(&self, idx: u32) -> Option<DataPtr> {
    let data: &Vec<T> = self;
    data.get(idx as usize).map(|r| r as *const _ as DataPtr)
  }

  fn set_value(&mut self, idx: u32, v: DataPtr) -> bool {
    if let Some(target) = self.get_mut(idx) {
      unsafe {
        let target = &mut *(target as *mut T);
        let source = &*(v as *const T);
        *target = (*source).clone()
      }

      true
    } else {
      false
    }
  }

  fn set_default_value(&mut self, idx: u32) -> bool {
    let value = T::default();
    self.set_value(idx, &value as *const _ as DataPtr)
  }

  unsafe fn grow_at_least(&mut self, max: usize) {
    let data: &mut Vec<T> = self;
    if data.len() <= max {
      data.resize(max + 1, T::default());
    }
  }
}
