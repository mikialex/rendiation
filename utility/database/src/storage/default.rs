use std::ops::DerefMut;

use crate::*;

/// The most common storage type that use a vec as the container.
/// Expecting dense distributed component data
pub struct DBLinearStorage<T> {
  pub data: Vec<T>,
  pub default_value: T,
  pub old_value_out: T, // todo, this value is leaked here, we should cleanup explicitly?
}

pub fn init_linear_storage<S: ComponentSemantic>() -> Arc<RwLock<DBLinearStorage<S::Data>>> {
  Arc::new(RwLock::new(DBLinearStorage::<S::Data> {
    data: Default::default(),
    default_value: S::default_override(),
    old_value_out: Default::default(),
  }))
}

impl<T: DataBaseDataType> ComponentStorage for Arc<RwLock<DBLinearStorage<T>>> {
  fn create_read_view(&self) -> Arc<dyn ComponentStorageReadView> {
    Arc::new(self.make_read_holder())
  }

  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView> {
    Box::new(self.make_write_holder())
  }
  fn type_id(&self) -> TypeId {
    TypeId::of::<T>()
  }
  fn data_shape(&self) -> &'static Shape<'_> {
    T::shape()
  }

  fn memory_usage_in_bytes(&self) -> usize {
    let data = self.read();
    (data.data.capacity() + 2) * std::mem::size_of::<T>()
  }
}

impl<T> ComponentStorageReadView for LockReadGuardHolder<DBLinearStorage<T>>
where
  T: DataBaseDataType,
{
  unsafe fn get(&self, idx: u32) -> DataPtr {
    let data: &Vec<T> = &self.data;
    data.get_unchecked(idx as usize) as *const _ as DataPtr
  }

  unsafe fn construct_dyn_datatype_from_raw_ptr<'a>(
    &self,
    ptr: DataPtr,
  ) -> &'a dyn DataBaseDataTypeDyn {
    let source = &*(ptr as *const T);
    source as &dyn DataBaseDataTypeDyn
  }
}

impl<T> ComponentStorageReadView for LockWriteGuardHolder<DBLinearStorage<T>>
where
  T: DataBaseDataType,
{
  unsafe fn get(&self, idx: u32) -> DataPtr {
    let data: &Vec<T> = &self.data;
    data.get_unchecked(idx as usize) as *const _ as DataPtr
  }

  unsafe fn construct_dyn_datatype_from_raw_ptr<'a>(
    &self,
    ptr: DataPtr,
  ) -> &'a dyn DataBaseDataTypeDyn {
    let source = &*(ptr as *const T);
    source as &dyn DataBaseDataTypeDyn
  }
}

impl<T> ComponentStorageReadWriteView for LockWriteGuardHolder<DBLinearStorage<T>>
where
  T: DataBaseDataType,
{
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

  unsafe fn set_value_from_small_serialize_data(
    &mut self,
    idx: u32,
    new_value: DBFastSerializeSmallBufferOrForeignKey<RawEntityHandle>,
  ) -> (DataPtr, DataPtr, bool) {
    let mut value = T::default();
    match new_value {
      DBFastSerializeSmallBufferOrForeignKey::Pod(small_vec) => {
        value.fast_deserialize(&mut small_vec.as_slice()).unwrap()
      }
      DBFastSerializeSmallBufferOrForeignKey::ForeignKey(handle) => {
        value = std::mem::transmute_copy(&handle)
      }
    }
    self.set_value(idx, Some(&value as *const _ as DataPtr))
  }

  unsafe fn delete(&mut self, idx: u32) -> DataPtr {
    let target = self.data.get_unchecked_mut(idx as usize);
    self.old_value_out = target.clone();
    &self.old_value_out as *const _ as DataPtr
  }

  fn resize(&mut self, max_address: u32) {
    let max_address = max_address as usize;
    if self.data.len() <= max_address {
      let default = self.default_value.clone();
      self.data.resize(max_address + 1, default);
    }
  }
}
