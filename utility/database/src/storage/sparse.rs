use std::ops::DerefMut;

use crate::*;

/// The most common storage type that use a vec as the container.
/// Expecting dense distributed component data
pub struct DBSparseStorage<T> {
  pub data: FastHashMap<u32, T>,
  pub default_value: T,
  pub old_value_out: T, // todo, this value is leaked here, we should cleanup explicitly?
}

pub fn init_sparse_storage<S: ComponentSemantic>() -> Arc<RwLock<DBSparseStorage<S::Data>>> {
  Arc::new(RwLock::new(DBSparseStorage::<S::Data> {
    data: Default::default(),
    default_value: S::default_override(),
    old_value_out: Default::default(),
  }))
}

impl<T: DataBaseDataType> ComponentStorage for Arc<RwLock<DBSparseStorage<T>>> {
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
    T::shape()
  }
}

impl<T> ComponentStorageReadView for LockReadGuardHolder<DBSparseStorage<T>>
where
  T: DataBaseDataType,
{
  fn get(&self, idx: u32) -> Option<DataPtr> {
    self
      .deref()
      .data
      .get(&idx)
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
    self.data.values().for_each(|data| {
      data.fast_serialize(&mut init).unwrap();
    });
    init
  }
}

impl<T> ComponentStorageReadWriteView for LockWriteGuardHolder<DBSparseStorage<T>>
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
    self.data.get(&idx).unwrap_unchecked() as *const _ as DataPtr
  }

  unsafe fn set_value(&mut self, idx: u32, new_value: Option<DataPtr>) -> (DataPtr, DataPtr, bool) {
    let self_ = self.deref_mut();
    let new = if let Some(new_value) = new_value {
      &*(new_value as *const T)
    } else {
      &self_.default_value
    };

    let old = self_.data.insert(idx, new.clone());

    let diff = if let Some(old) = old {
      self_.old_value_out = old;
      &self_.old_value_out == new
    } else {
      true
    };

    let new = new as *const _ as DataPtr;
    let old = &self.old_value_out as *const _ as DataPtr;
    (new, old, diff)
  }

  unsafe fn delete(&mut self, idx: u32) -> DataPtr {
    let target = self.data.remove(&idx).unwrap_unchecked();
    self.old_value_out = target.clone();
    &self.old_value_out as *const _ as DataPtr
  }

  fn resize(&mut self, _max_address: u32) {
    // noop, because it's the sparse storage
  }
}
