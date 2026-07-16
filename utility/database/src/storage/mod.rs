use crate::*;
mod default;
mod sparse;
mod validator_storage_caller;
#[cfg(test)]
mod validator_storage_impl;

pub use default::*;
pub use sparse::*;
pub use validator_storage_caller::*;

/// A raw pointer to data of type `T` stored in a database data container.
pub(crate) type DataPtr = *const ();
/// Mutable counterpart of [DataPtr].
pub(crate) type DataMutPtr = *mut ();

pub trait DataBaseDataType: CValue + Default {
  /// Serialize `self` into a byte-oriented output.
  ///
  /// Implementations are free to choose any encoding, but efficient binary
  /// formats are recommended for performance.
  #[must_use]
  fn serialize_to_writer(&self, target: &mut (impl std::io::Write + ?Sized)) -> Option<()>;
  /// Deserialize `self` from a byte-oriented input, replacing the current value.
  ///
  /// Implementations are free to choose any encoding, but efficient binary
  /// formats are recommended for performance.
  #[must_use]
  fn deserialize_from_reader(&mut self, source: &mut (impl std::io::Read + ?Sized)) -> Option<()>;
  fn shape() -> &'static facet::Shape<'static>;
}

pub type DatabaseSerializedFieldBuffer = smallvec::SmallVec<[u8; 16]>;

/// Build a closure that deserializes a serialized field buffer
/// into the concrete `DataBaseDataType` and formats it via `Debug`.
/// Returns `None` if deserialization fails.
pub fn create_binary_to_debug_string<T: DataBaseDataType>(
) -> fn(DatabaseSerializedFieldBuffer) -> Option<String> {
  |buffer| {
    let mut value = T::default();
    let mut cursor = std::io::Cursor::new(buffer.as_ref());
    if T::deserialize_from_reader(&mut value, &mut cursor).is_some() {
      Some(format!("{:?}", value))
    } else {
      None
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum DatabaseSerializedFieldBufferOrForeignKey {
  Pod(DatabaseSerializedFieldBuffer),
  ForeignKey(RawEntityHandle),
}

pub trait DynDataBaseDataType {
  fn serialize_to_writer_dyn(&self, target: &mut dyn std::io::Write) -> Option<()>;
  fn deserialize_from_reader_dyn(&mut self, source: &mut dyn std::io::Read) -> Option<()>;

  fn serialize_into_buffer(&self) -> DatabaseSerializedFieldBuffer {
    let mut target = smallvec::SmallVec::new();
    self.serialize_to_writer_dyn(&mut target);
    target.shrink_to_fit();
    target
  }

  fn shape(&self) -> &'static facet::Shape<'static>;
  /// this function will be removed in the future.
  fn debug_value(&self) -> String;
}

impl<T: DataBaseDataType> DynDataBaseDataType for T {
  fn serialize_to_writer_dyn(&self, target: &mut dyn std::io::Write) -> Option<()> {
    self.serialize_to_writer(target)
  }

  fn deserialize_from_reader_dyn(&mut self, source: &mut dyn std::io::Read) -> Option<()> {
    self.deserialize_from_reader(source)
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
  fn serialize_to_writer(&self, writer: &mut (impl std::io::Write + ?Sized)) -> Option<()> {
    rmp_serde::encode::write(writer, self).ok()
  }

  fn deserialize_from_reader(&mut self, reader: &mut (impl std::io::Read + ?Sized)) -> Option<()> {
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

/// A struct that stores a type T's metadata and useful functions.
#[derive(Clone)]
pub struct DataTypeMetaInfo {
  /// The DataPtr passed in must be `&T`. Returns a dyn pointer whose lifetime matches the input DataPtr (`&T`).
  pub create_dyn_data_ptr_readonly: unsafe fn(DataPtr) -> *const dyn DynDataBaseDataType,
  /// Same as [Self::create_dyn_data_ptr_readonly], but returns a mutable dyn pointer.
  pub create_dyn_data_ptr: unsafe fn(DataMutPtr) -> *mut dyn DynDataBaseDataType,
  /// The reflection info of `T`, using the facet crate.
  pub shape: &'static facet::Shape<'static>,
  /// `T`'s [std::any::TypeId].
  pub data_type_id: TypeId,
}

impl DataTypeMetaInfo {
  /// # Safety
  ///
  /// The impl must ensure the returned pointer points to the correct data.
  #[inline(always)]
  pub unsafe fn construct_dyn_datatype_from_raw_ptr<'a>(
    &self,
    ptr: DataPtr,
  ) -> &'a dyn DynDataBaseDataType {
    &*(self.create_dyn_data_ptr_readonly)(ptr) as &dyn DynDataBaseDataType
  }

  /// Construct a [DataTypeMetaInfo] for a type T that implements [DataBaseDataType].
  pub fn from_type<T: DataBaseDataType>() -> Self {
    Self {
      create_dyn_data_ptr_readonly: |ptr| unsafe {
        &*(ptr as *const T) as *const dyn DynDataBaseDataType
      },
      create_dyn_data_ptr: |ptr| unsafe { &mut *(ptr as *mut T) as *mut dyn DynDataBaseDataType },
      shape: T::shape(),
      data_type_id: TypeId::of::<T>(),
    }
  }
}

/// Encapsulates the underlying storage strategy for a component.
/// Different components may benefit from different storage implementations:
///
/// - *Sparse* data: use a hashmap to reduce memory at the cost of access speed.
/// - *Mutually exclusive*: when multiple component types are mutually exclusive
///   (i.e., an entity has at most one of them at a time), a single storage can hold
///   all of them together in an enum-like buffer to save space.
/// - *Co-accessed*: when multiple component types are always accessed together in
///   the same code paths, a single storage can hold them in an interleaved AOS
///   (array-of-structs) buffer to improve cache locality.
///
/// The component storage is addressed by a linear (starting from zero),
/// re-allocatable (same index can be recycled) u32 index.
pub trait ComponentStorage: Send + Sync + DynClone {
  fn create_read_view(&self) -> ComponentReadViewBox;
  fn create_read_write_view(&self) -> ComponentReadWriteViewBox;
  fn create_meta(&self) -> DataTypeMetaInfo;

  fn memory_usage_in_bytes(&self) -> usize;
}
dyn_clone::clone_trait_object!(ComponentStorage);

pub trait ComponentStorageReadViewBase: Send + Sync {
  fn meta(&self) -> &DataTypeMetaInfo;
  /// # Safety
  ///  
  /// - the caller must ensure that idx is valid
  /// - the impl must ensure the returned ptr is point to correct data
  ///
  /// get the data located in idx
  unsafe fn get(&self, idx: u32) -> DataPtr;

  /// # Safety
  ///  
  /// - the caller must ensure that idx is valid
  /// - the impl must ensure the returned ptr is point to correct data
  ///
  /// get the data located in idx
  #[inline(always)]
  unsafe fn get_as_dyn_storage(&self, idx: u32) -> &dyn DynDataBaseDataType {
    self
      .meta()
      .construct_dyn_datatype_from_raw_ptr(self.get(idx))
  }
}

pub trait ComponentStorageReadView: ComponentStorageReadViewBase {
  fn clone_boxed(&self) -> ComponentReadViewBox;
}

pub trait ComponentStorageReadWriteView: ComponentStorageReadViewBase {
  /// # Safety
  ///
  /// The caller must ensure that:
  ///
  /// - The idx must not be in use (previously initialized same index must be deleted).
  /// - The idx is in the allocated range (range controlled by [Self::resize]).
  ///
  /// The implementation must ensure that:
  ///
  /// - If init_value is None, then the implementation should use T's default value as init value.
  ///   - T's default is defined by [ComponentSemantic::default_override]
  /// - The init copies the data and returns a pointer that points to the data instance
  ///   living in the database.
  ///   - The returned pointer is temporarily valid before the next mutation (fn call in this trait).
  unsafe fn set_value_init(&mut self, idx: u32, init_value: Option<DataPtr>) -> DataPtr;

  /// # Safety
  ///
  /// The caller must ensure that:
  ///
  /// - The idx is living. (living index is created by [Self::set_value_init])
  ///
  /// The implementation must ensure that:
  ///
  /// - The write will do the data copy, return (new_ptr, old_ptr, changed)
  ///   -  the returned ptr is temporally valid before next mutation
  ///   - the new_ptr points to the data instance living in the database.
  ///   - the old_ptr points to the old data instance living in the database
  ///     - note, this ptr is not the data instance before writing(it may be copied from the old ptr into somewhere)
  ///   - change info is compute by [ComponentSemantic::Data]'s [PartialEq] implementation
  unsafe fn set_value(&mut self, idx: u32, new_value: DataPtr) -> (DataPtr, DataPtr, bool);

  /// # Safety
  /// ditto [Self::set_value]
  unsafe fn set_value_from_serialize_field_data(
    &mut self,
    idx: u32,
    new_value: DatabaseSerializedFieldBufferOrForeignKey,
  ) -> (DataPtr, DataPtr, bool);

  /// # Safety
  ///  
  /// - the caller must ensure that idx is valid
  /// - the idx must point to living location, return old value ptr
  unsafe fn delete(&mut self, idx: u32) -> DataPtr;

  /// Resize the storage to allow more data to be stored at the bound of the max size address.
  /// or shrink the storage to save memory
  ///
  /// # Safety
  ///
  /// Caller must ensure that max_address covers all living indices.
  /// The container's default max_address is 0.
  unsafe fn resize(&mut self, max_address: u32);

  /// Clean up the transient old-value object that the implementation may hold
  /// internally (e.g. `old_value_out`). After calling this, any previously
  /// returned old `DataPtr` from [set_value] or [delete] is no longer valid.
  ///
  /// The database never calls this automatically because resetting the old-value
  /// storage has a cost (it may involve a `T::default()` allocation/drop) that
  /// every mutation does not need. Call this only when:
  ///
  /// - The mutation rate is known to be low, or
  /// - The old value held significant heap memory (e.g. large `Vec`, `String`)
  ///   that you want to release eagerly instead of waiting for the next mutation (fn call in this trait)
  ///   to overwrite it.
  fn cleanup_possible_old_ptr_transient_object(&mut self);
}
