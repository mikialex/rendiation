use std::ops::DerefMut;

use crate::*;

/// A dense storage using a `Vec` indexed by entity allocation index. Suitable
/// for components that most entities hold (the common case).
pub struct DBLinearStorage<T> {
  pub data: Vec<T>,
  pub default_value: T,
  pub old_value_out: T,
  pub meta: DataTypeMetaInfo,
}

pub fn init_linear_storage<S: ComponentSemantic>() -> Arc<RwLock<DBLinearStorage<S::Data>>> {
  Arc::new(RwLock::new(DBLinearStorage::<S::Data> {
    data: Default::default(),
    default_value: S::default_override(),
    old_value_out: Default::default(),
    meta: DataTypeMetaInfo::from_type::<S::Data>(),
  }))
}

impl<T: DataBaseDataType> ComponentStorage for Arc<RwLock<DBLinearStorage<T>>> {
  fn create_meta(&self) -> DataTypeMetaInfo {
    self.read().meta.clone()
  }

  fn create_read_view(&self) -> ComponentReadViewBox {
    smallbox!(self.make_read_holder())
  }

  fn create_read_write_view(&self) -> ComponentReadWriteViewBox {
    smallbox!(self.make_write_holder())
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
  fn clone_boxed(&self) -> ComponentReadViewBox {
    smallbox!(self.clone())
  }
}
impl<T> ComponentStorageReadViewBase for LockReadGuardHolder<DBLinearStorage<T>>
where
  T: DataBaseDataType,
{
  #[inline(always)]
  fn meta(&self) -> &DataTypeMetaInfo {
    &self.meta
  }

  #[inline(always)]
  unsafe fn get(&self, idx: u32) -> DataPtr {
    let data: &Vec<T> = &self.data;
    data.get_unchecked(idx as usize) as *const _ as DataPtr
  }
}

impl<T> ComponentStorageReadViewBase for LockWriteGuardHolder<DBLinearStorage<T>>
where
  T: DataBaseDataType,
{
  #[inline(always)]
  fn meta(&self) -> &DataTypeMetaInfo {
    &self.meta
  }
  #[inline(always)]
  unsafe fn get(&self, idx: u32) -> DataPtr {
    let data: &Vec<T> = &self.data;
    data.get_unchecked(idx as usize) as *const _ as DataPtr
  }
}

impl<T> ComponentStorageReadWriteView for LockWriteGuardHolder<DBLinearStorage<T>>
where
  T: DataBaseDataType,
{
  unsafe fn set_value_init(&mut self, idx: u32, init_value: Option<DataPtr>) -> DataPtr {
    let self_ = self.deref_mut();

    let source = if let Some(new_value) = init_value {
      &*(new_value as *const T)
    } else {
      &self_.default_value
    };

    let target = self_.data.get_unchecked_mut(idx as usize);
    *target = (*source).clone();

    target as *const _ as DataPtr
  }

  unsafe fn set_value(&mut self, idx: u32, source: DataPtr) -> (DataPtr, DataPtr, bool) {
    let self_ = self.deref_mut();
    let target = self_.data.get_unchecked_mut(idx as usize);

    let source = &*(source as *const T);

    self_.old_value_out = target.clone();
    *target = (*source).clone();

    let diff = &self_.old_value_out != target;

    let new = target as *const _ as DataPtr;
    let old = &self.old_value_out as *const _ as DataPtr;
    (new, old, diff)
  }

  unsafe fn set_value_from_serialize_field_data(
    &mut self,
    idx: u32,
    new_value: DatabaseSerializedFieldBufferOrForeignKey,
  ) -> (DataPtr, DataPtr, bool) {
    let mut value = T::default();
    match new_value {
      DatabaseSerializedFieldBufferOrForeignKey::Pod(small_vec) => value
        .deserialize_from_reader(&mut small_vec.as_slice())
        .unwrap(),
      DatabaseSerializedFieldBufferOrForeignKey::ForeignKey(handle) => {
        value = std::mem::transmute_copy(&Some(handle))
      }
    }
    self.set_value(idx, &value as *const _ as DataPtr)
  }

  unsafe fn delete(&mut self, idx: u32) -> DataPtr {
    let target = self.data.get_unchecked_mut(idx as usize);
    self.old_value_out = target.clone();
    &self.old_value_out as *const _ as DataPtr
  }

  unsafe fn resize(&mut self, max_address: u32) {
    let default = self.default_value.clone();
    self.data.resize(max_address as usize + 1, default);
    self.data.shrink_to_fit();
  }

  fn cleanup_possible_old_ptr_transient_object(&mut self) {
    let self_ = self.deref_mut();
    self_.old_value_out = T::default();
  }
}
