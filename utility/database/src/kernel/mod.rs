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
