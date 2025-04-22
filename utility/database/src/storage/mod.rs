use crate::*;
mod default;

pub use default::*;

pub type DataPtr = *const ();
pub type DataMutPtr = *const ();

pub trait DataBaseDataType: CValue + Default {
  #[must_use]
  fn fast_serialize(&self, target: &mut (impl std::io::Write + ?Sized)) -> Option<()>;
  #[must_use]
  fn fast_deserialize(&mut self, source: &mut (impl std::io::Read + ?Sized)) -> Option<()>;
  fn shape() -> &'static facet::Shape;
}

pub trait DataBaseDataTypeDyn {
  fn fast_serialize_dyn(&self, target: &mut dyn std::io::Write) -> Option<()>;
  fn fast_deserialize_dyn(&mut self, source: &mut dyn std::io::Read) -> Option<()>;
  fn shape(&self) -> &'static facet::Shape;
  /// this function will be removed in future.
  fn debug_value(&self) -> String;
}

impl<T: DataBaseDataType> DataBaseDataTypeDyn for T {
  fn fast_serialize_dyn(&self, target: &mut dyn std::io::Write) -> Option<()> {
    self.fast_serialize(target)
  }

  fn fast_deserialize_dyn(&mut self, source: &mut dyn std::io::Read) -> Option<()> {
    self.fast_deserialize(source)
  }

  fn shape(&self) -> &'static facet::Shape {
    T::shape()
  }

  fn debug_value(&self) -> String {
    format!("{:#?}", self)
  }
}

impl<T> DataBaseDataType for T
where
  T: CValue + Default,
  // T: Facet,
  T: Serialize + for<'a> Deserialize<'a>,
{
  fn fast_serialize(&self, writer: &mut (impl std::io::Write + ?Sized)) -> Option<()> {
    rmp_serde::encode::write(writer, self).ok()
  }

  fn fast_deserialize(&mut self, reader: &mut (impl std::io::Read + ?Sized)) -> Option<()> {
    *self = rmp_serde::from_read(reader).ok()?;
    Some(())
  }

  fn shape() -> &'static facet::Shape {
    unimplemented!()
  }
}

/// This trait encapsulate the implementation of component storage.
/// For different kinds of component, we can have different storage implementation.
/// For example. If the component data is sparse, we could using hashmap as the storage
/// to improve the space efficiency at the cost of access performance. If the multiple
/// component data is exclusively exists, we can use a enum like buffer to improve the
/// space efficiency. If the multiple component will always accessed together, we could
/// store them in a interleaved buffer like common AOS way to improve the access performance.
pub trait ComponentStorage: Send + Sync + DynClone {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView>;
  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView>;
  fn type_id(&self) -> TypeId;
  fn data_shape(&self) -> &'static facet::Shape;
}
dyn_clone::clone_trait_object!(ComponentStorage);

pub trait ComponentStorageReadView: Send + Sync + DynClone {
  /// get the data located in idx, return None if out of bound.
  fn get(&self, idx: u32) -> Option<DataPtr>;

  /// # Safety
  /// ptr must be the correct data type.
  unsafe fn construct_dyn_datatype_from_raw_ptr<'a>(
    &self,
    ptr: DataPtr,
  ) -> &'a dyn DataBaseDataTypeDyn;

  fn get_as_dyn_storage(&self, idx: u32) -> Option<&dyn DataBaseDataTypeDyn> {
    self
      .get(idx)
      .map(|ptr| unsafe { self.construct_dyn_datatype_from_raw_ptr(ptr) })
  }
  fn fast_serialize_all(&self) -> Option<Vec<u8>>;
}
dyn_clone::clone_trait_object!(ComponentStorageReadView);

pub trait ComponentStorageReadWriteView {
  /// # Safety
  /// ptr must be the correct data type.
  unsafe fn construct_dyn_datatype_from_raw_ptr<'a>(
    &self,
    ptr: DataPtr,
  ) -> &'a dyn DataBaseDataTypeDyn;

  /// # Safety
  ///
  /// idx must point to living data
  ///
  /// get the data located in idx.
  unsafe fn get(&self, idx: u32) -> DataPtr;

  /// # Safety
  ///
  /// idx must point to living data
  ///
  /// get the data in dynamic form located in idx.
  unsafe fn get_as_dyn_storage(&self, idx: u32) -> &dyn DataBaseDataTypeDyn {
    self.construct_dyn_datatype_from_raw_ptr(self.get(idx))
  }

  /// # Safety
  /// The index must point to living data if old_value_out is Some, otherwise it must be pointer to
  /// an allocate but not used location. Return (new_value_ptr, old_value_ptr, changed_if_not_init)
  ///
  /// if new_value is None, then, new value should use default value
  unsafe fn set_value(&mut self, idx: u32, new_value: Option<DataPtr>) -> (DataPtr, DataPtr, bool);

  /// # Safety
  /// the idx must point to living location, return old value ptr
  unsafe fn delete(&mut self, idx: u32) -> DataPtr;

  /// grow the storage to allow more data to stored at the bound of the max size address.
  fn grow(&mut self, max: u32);

  fn fast_deserialize_all(&mut self, source: &[u8], count: usize);
}
