use std::ops::DerefMut;

use crate::*;

/// The most common storage type that use a vec as the container.
/// Expecting dense distributed component data
pub struct DBDefaultLinearStorage<T> {
  pub data: Vec<T>,
  pub default_value: T,
  pub old_value_out: T, // todo, this value is leaked here, we should cleanup explicitly?
}

impl<T: DataBaseDataType> ComponentStorage for Arc<RwLock<DBDefaultLinearStorage<T>>> {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView> {
    Box::new(self.make_read_holder())
  }

  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView> {
    Box::new(self.make_write_holder())
  }
  fn type_id(&self) -> TypeId {
    TypeId::of::<T>()
  }
  fn data_shape(&self) -> &'static Shape {
    // T::SHAPE
    unimplemented!()
  }
}

impl<T> ComponentStorageReadView for LockReadGuardHolder<DBDefaultLinearStorage<T>>
where
  T: DataBaseDataType,
{
  fn get(&self, idx: u32) -> Option<DataPtr> {
    self
      .deref()
      .data
      .get(idx as usize)
      .map(|r| r as *const _ as DataPtr)
  }

  unsafe fn construct_dyn_datatype_from_raw_ptr<'a>(
    &self,
    ptr: DataPtr,
  ) -> &'a dyn DataBaseDataTypeDyn {
    let source = &*(ptr as *const T);
    source as &dyn DataBaseDataTypeDyn
  }

  fn fast_serialize_all(&self) -> Vec<u8> {
    let mut init = Vec::<u8>::with_capacity(self.data.len() * std::mem::size_of::<T>());
    self.data.iter().for_each(|data| {
      data.fast_serialize(&mut init);
    });
    init
  }
}

impl<T> ComponentStorageReadWriteView for LockWriteGuardHolder<DBDefaultLinearStorage<T>>
where
  T: DataBaseDataType,
{
  unsafe fn construct_dyn_datatype_from_raw_ptr<'a>(
    &self,
    ptr: DataPtr,
  ) -> &'a dyn DataBaseDataTypeDyn {
    let source = &*(ptr as *const T);
    source as &dyn DataBaseDataTypeDyn
  }

  unsafe fn get(&self, idx: u32) -> DataPtr {
    let data: &Vec<T> = &self.data;
    data.get_unchecked(idx as usize) as *const _ as DataPtr
  }

  unsafe fn set_value(&mut self, idx: u32, new_value: Option<DataPtr>) -> (DataPtr, DataPtr, bool) {
    let self_ = self.deref_mut();
    let target = self_.data.get_unchecked_mut(idx as usize);
    let source = if let Some(new_value) = new_value {
      &*(new_value as *const T)
    } else {
      &self_.default_value
    };

    self_.old_value_out = target.clone();
    *target = (*source).clone();

    let diff = &self_.old_value_out != target;

    let new = target as *const _ as DataPtr;
    let old = &self.old_value_out as *const _ as DataPtr;
    (new, old, diff)
  }

  unsafe fn delete(&mut self, idx: u32) -> DataPtr {
    let target = self.data.get_unchecked_mut(idx as usize);
    self.old_value_out = target.clone();
    &self.old_value_out as *const _ as DataPtr
  }

  fn grow(&mut self, max: u32) {
    let max = max as usize;
    if self.data.len() <= max {
      let default = self.default_value.clone();
      self.data.resize(max + 1, default);
    }
  }

  fn fast_deserialize_all(&mut self, _: &[u8], _: usize) {
    //
  }
}
