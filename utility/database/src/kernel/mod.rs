mod component;
mod ecg;
mod entity_writer;
mod entry;
mod lock;

use std::marker::PhantomData;

pub use component::*;
pub use ecg::*;
pub use entity_writer::*;
pub use entry::*;
pub use lock::*;
use reactive::ValueChange;

pub struct SendSyncPhantomData<T> {
  phantom: PhantomData<T>,
}
unsafe impl<T> Send for SendSyncPhantomData<T> {}
unsafe impl<T> Sync for SendSyncPhantomData<T> {}

impl<T> Clone for SendSyncPhantomData<T> {
  fn clone(&self) -> Self {
    *self
  }
}
impl<T> Copy for SendSyncPhantomData<T> {}
impl<T> Default for SendSyncPhantomData<T> {
  fn default() -> Self {
    Self {
      phantom: Default::default(),
    }
  }
}

pub enum ScopedMessage<T> {
  Start,
  End,
  Message(T),
}

pub type ScopedValueChange<T> = ScopedMessage<IndexValueChange<T>>;
pub type EntityRangeChange = ScopedValueChange<()>;

pub struct IndexValueChange<T> {
  pub idx: u32,
  pub change: ValueChange<T>,
}

use std::sync::Arc;
pub struct ExternalRefPtr<T> {
  pub ptr: Arc<T>,
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
