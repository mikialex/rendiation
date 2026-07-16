use std::ops::DerefMut;

use crate::*;

/// A sparse storage using a HashMap to only allocate for entities that actually
/// hold this component. Suitable for components that are sparsely distributed
/// (i.e., only a subset of entities have them).
///
/// Values equal to the component's default are not stored in the map, so writing
/// the default back effectively reclaims the entry's space.
pub struct DBSparseStorage<T> {
  pub data: FastHashMap<u32, T>,
  pub default_value: T,
  pub old_value_out: T,
  pub meta: DataTypeMetaInfo,
}

pub fn init_sparse_storage<S: ComponentSemantic>() -> Arc<RwLock<DBSparseStorage<S::Data>>> {
  init_sparse_storage_by_data::<S::Data>(S::default_override())
}

pub fn init_sparse_storage_by_data<T: DataBaseDataType>(
  default_value: T,
) -> Arc<RwLock<DBSparseStorage<T>>> {
  Arc::new(RwLock::new(DBSparseStorage::<T> {
    data: Default::default(),
    default_value,
    old_value_out: Default::default(),
    meta: DataTypeMetaInfo::from_type::<T>(),
  }))
}

impl<T: DataBaseDataType> ComponentStorage for Arc<RwLock<DBSparseStorage<T>>> {
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

impl<T> ComponentStorageReadView for LockReadGuardHolder<DBSparseStorage<T>>
where
  T: DataBaseDataType,
{
  fn clone_boxed(&self) -> ComponentReadViewBox {
    smallbox!(self.clone())
  }
}

impl<T> ComponentStorageReadViewBase for LockReadGuardHolder<DBSparseStorage<T>>
where
  T: DataBaseDataType,
{
  fn meta(&self) -> &DataTypeMetaInfo {
    &self.meta
  }
  unsafe fn get(&self, idx: u32) -> DataPtr {
    self.data.get(&idx).unwrap_or(&self.default_value) as *const _ as DataPtr
  }
}

impl<T> ComponentStorageReadViewBase for LockWriteGuardHolder<DBSparseStorage<T>>
where
  T: DataBaseDataType,
{
  fn meta(&self) -> &DataTypeMetaInfo {
    &self.meta
  }
  unsafe fn get(&self, idx: u32) -> DataPtr {
    self.data.get(&idx).unwrap_or(&self.default_value) as *const _ as DataPtr
  }
}

impl<T> ComponentStorageReadWriteView for LockWriteGuardHolder<DBSparseStorage<T>>
where
  T: DataBaseDataType,
{
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

  unsafe fn set_value_init(&mut self, idx: u32, init_value: Option<DataPtr>) -> DataPtr {
    let self_ = self.deref_mut();

    if let Some(new_value) = init_value {
      let new_value = &*(new_value as *const T);
      if new_value == &self_.default_value {
        &self_.default_value as *const _ as DataPtr
      } else {
        let entry = self_.data.entry(idx).insert(new_value.clone());
        entry.get() as *const _ as DataPtr
      }
    } else {
      &self_.default_value as *const _ as DataPtr
    }
  }

  unsafe fn set_value(&mut self, idx: u32, new_value: DataPtr) -> (DataPtr, DataPtr, bool) {
    let self_ = self.deref_mut();
    let new = &*(new_value as *const T);

    if new == &self_.default_value {
      let old = self_.data.remove(&idx);

      let diff = if let Some(old) = old {
        self_.old_value_out = old;
        &self_.old_value_out != new
      } else {
        false
      };

      let new = &self_.default_value as *const _ as DataPtr;
      let old = &self.old_value_out as *const _ as DataPtr;
      (new, old, diff)
    } else {
      let old = self_.data.insert(idx, new.clone());

      let diff = if let Some(old) = old {
        self_.old_value_out = old;
        &self_.old_value_out != new
      } else {
        true
      };

      let new = self_.data.get(&idx).unwrap() as *const _ as DataPtr;
      let old = &self.old_value_out as *const _ as DataPtr;
      (new, old, diff)
    }
  }

  unsafe fn delete(&mut self, idx: u32) -> DataPtr {
    if let Some(target) = self.data.remove(&idx) {
      self.old_value_out = target.clone();
      &self.old_value_out as *const _ as DataPtr
    } else {
      &self.default_value as *const _ as DataPtr
    }
  }

  unsafe fn resize(&mut self, _max_address: u32) {
    // noop, because it's the sparse storage
  }

  fn cleanup_possible_old_ptr_transient_object(&mut self) {
    let self_ = self.deref_mut();
    self_.old_value_out = T::default();
  }
}
