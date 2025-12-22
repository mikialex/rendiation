use crate::*;
mod default;
mod sparse;

pub use default::*;
pub use sparse::*;

pub type DataPtr = *const ();
pub type DataMutPtr = *const ();

pub trait DataBaseDataType: CValue + Default {
  #[must_use]
  fn fast_serialize(&self, target: &mut (impl std::io::Write + ?Sized)) -> Option<()>;
  #[must_use]
  fn fast_deserialize(&mut self, source: &mut (impl std::io::Read + ?Sized)) -> Option<()>;
  fn shape() -> &'static facet::Shape<'static>;
}

pub type DBFastSerializeSmallBuffer = smallvec::SmallVec<[u8; 16]>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum DBFastSerializeSmallBufferOrForeignKey<K> {
  Pod(DBFastSerializeSmallBuffer),
  ForeignKey(K),
}

impl<K> DBFastSerializeSmallBufferOrForeignKey<K> {
  pub fn map<K2>(self, f: impl FnOnce(K) -> K2) -> DBFastSerializeSmallBufferOrForeignKey<K2> {
    use DBFastSerializeSmallBufferOrForeignKey::*;
    match self {
      Pod(p) => Pod(p),
      ForeignKey(k) => ForeignKey(f(k)),
    }
  }
}

pub trait DataBaseDataTypeDyn {
  fn fast_serialize_dyn(&self, target: &mut dyn std::io::Write) -> Option<()>;
  fn fast_deserialize_dyn(&mut self, source: &mut dyn std::io::Read) -> Option<()>;

  fn fast_serialize_into_buffer(&self) -> DBFastSerializeSmallBuffer {
    let mut target = smallvec::SmallVec::new();
    self.fast_serialize_dyn(&mut target);
    target.shrink_to_fit();
    target
  }

  fn shape(&self) -> &'static facet::Shape<'static>;
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

  fn shape(&self) -> &'static facet::Shape<'static> {
    T::shape()
  }

  fn debug_value(&self) -> String {
    format!("{:#?}", self)
  }
}

impl<T> DataBaseDataType for T
where
  T: CValue + Default,
  T: Facet<'static>,
  T: Serialize + for<'a> Deserialize<'a>,
{
  fn fast_serialize(&self, writer: &mut (impl std::io::Write + ?Sized)) -> Option<()> {
    rmp_serde::encode::write(writer, self).ok()
  }

  fn fast_deserialize(&mut self, reader: &mut (impl std::io::Read + ?Sized)) -> Option<()> {
    *self = rmp_serde::from_read(reader).ok()?;
    Some(())
  }

  fn shape() -> &'static facet::Shape<'static> {
    T::SHAPE
  }
}

pub type ComponentReadViewBox = SmallBox<dyn ComponentStorageReadView, smallbox::space::S2>;
pub type ComponentReadWriteViewBox =
  SmallBox<dyn ComponentStorageReadWriteView, smallbox::space::S2>;

#[test]
fn common_view_should_fit_in_view_box() {
  declare_entity!(T);
  declare_component!(TC, T, u32);

  let storage = init_linear_storage::<TC>();
  let view = storage.create_read_view();
  assert!(!view.is_heap());
  drop(view);
  let view = storage.create_read_write_view();
  assert!(!view.is_heap());
  drop(view);

  let storage = init_sparse_storage::<TC>();
  let view = storage.create_read_view();
  assert!(!view.is_heap());
  drop(view);
  let view = storage.create_read_write_view();
  assert!(!view.is_heap());
  drop(view);
}

/// This trait encapsulate the implementation of component storage.
/// For different kinds of component, we can have different storage implementation.
/// For example. If the component data is sparse, we could using hashmap as the storage
/// to improve the space efficiency at the cost of access performance. If the multiple
/// component data is exclusively exists, we can use a enum like buffer to improve the
/// space efficiency. If the multiple component will always accessed together, we could
/// store them in a interleaved buffer like common AOS way to improve the access performance.
pub trait ComponentStorage: Send + Sync + DynClone {
  fn create_read_view(&self) -> ComponentReadViewBox;
  fn create_read_write_view(&self) -> ComponentReadWriteViewBox;
  fn type_id(&self) -> TypeId;
  fn data_shape(&self) -> &'static facet::Shape<'_>;

  fn memory_usage_in_bytes(&self) -> usize;
}
dyn_clone::clone_trait_object!(ComponentStorage);

pub trait ComponentStorageReadViewBase: Send + Sync {
  // fn clone_boxed(&self) -> ComponentReadViewBox;

  /// # Safety
  ///  
  /// - the caller must ensure that idx is valid
  /// - the impl must ensure the returned ptr is point to correct data
  ///
  /// get the data located in idx
  unsafe fn get(&self, idx: u32) -> DataPtr;

  /// # Safety
  ///
  /// the impl must ensure the returned ptr is point to correct data
  unsafe fn construct_dyn_datatype_from_raw_ptr<'a>(
    &self,
    ptr: DataPtr,
  ) -> &'a dyn DataBaseDataTypeDyn;

  /// # Safety
  ///  
  /// - the caller must ensure that idx is valid
  /// - the impl must ensure the returned ptr is point to correct data
  ///
  /// get the data located in idx
  unsafe fn get_as_dyn_storage(&self, idx: u32) -> &dyn DataBaseDataTypeDyn {
    self.construct_dyn_datatype_from_raw_ptr(self.get(idx))
  }
}

pub trait ComponentStorageReadView: ComponentStorageReadViewBase {
  fn clone_boxed(&self) -> ComponentReadViewBox;
}

pub trait ComponentStorageReadWriteView: ComponentStorageReadViewBase {
  /// # Safety
  /// The index must point to living data if old_value_out is Some, otherwise it must be pointer to
  /// an allocate but not used location. Return (new_value_ptr, old_value_ptr, changed_if_not_init)
  ///
  /// if new_value is None, then, new value should use default value
  unsafe fn set_value(&mut self, idx: u32, new_value: Option<DataPtr>) -> (DataPtr, DataPtr, bool);

  /// # Safety
  /// ditto
  unsafe fn set_value_from_small_serialize_data(
    &mut self,
    idx: u32,
    new_value: DBFastSerializeSmallBufferOrForeignKey<RawEntityHandle>,
  ) -> (DataPtr, DataPtr, bool);

  /// # Safety
  ///  
  /// - the caller must ensure that idx is valid
  /// - the idx must point to living location, return old value ptr
  unsafe fn delete(&mut self, idx: u32) -> DataPtr;

  /// resize the storage to allow more data to stored at the bound of the max size address.
  /// or shrink the storage to save memory
  fn resize(&mut self, max_address: u32);
}
