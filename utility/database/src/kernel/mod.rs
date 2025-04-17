mod component;
mod component_typed;
mod ecg;
mod entity_reader;
mod entity_writer;
mod entity_writer_typed;
mod entry;
mod handle;
mod lock;
mod query;
mod writer_value_init;

pub use component::*;
pub use component_typed::*;
pub use ecg::*;
pub use entity_reader::*;
pub use entity_writer::*;
pub use entity_writer_typed::*;
pub use entry::*;
pub use handle::*;
pub use lock::*;
pub use query::*;
pub use writer_value_init::*;

use crate::*;

/// The message event sequence is scoped inside the pair of `Start` and `End`
/// to provide additional information to the observer of the change. So that the
/// observer may optimize performance by skipping the lock accessing inside the pair.
pub enum ScopedMessage<T> {
  Start,
  End,
  Message(T),
}

pub type ScopedValueChange<T> = ScopedMessage<IndexValueChange<T>>;

pub struct IndexValueChange<T> {
  pub idx: RawEntityHandle,
  pub change: ValueChange<T>,
}

/// This struct use ptr equality as PartialEq impl compare to Arc
///
/// User should use this instead of Arc to ensure good performance in delta propagation
#[derive(Default, Facet)]
pub struct ExternalRefPtr<T> {
  pub ptr: Arc<T>,
}

impl<T> ExternalRefPtr<T> {
  pub fn new(data: T) -> Self {
    Self {
      ptr: Arc::new(data),
    }
  }
  pub fn new_shared(data: Arc<T>) -> Self {
    Self { ptr: data }
  }
}

impl<T> std::ops::Deref for ExternalRefPtr<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.ptr.deref()
  }
}

impl<T> Clone for ExternalRefPtr<T> {
  fn clone(&self) -> Self {
    Self {
      ptr: self.ptr.clone(),
    }
  }
}
impl<T> PartialEq for ExternalRefPtr<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.ptr, &other.ptr)
  }
}
impl<T> std::fmt::Debug for ExternalRefPtr<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ExternalRefPtr")
      .field("ptr", &(Arc::as_ptr(&self.ptr) as *const u8))
      .finish()
  }
}

pub trait EntityCustomWrite<E: EntitySemantic> {
  type Writer;
  fn create_writer() -> Self::Writer;
  fn write(self, writer: &mut Self::Writer) -> EntityHandle<E>;
}
