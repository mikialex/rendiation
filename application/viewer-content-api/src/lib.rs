use core::slice;
use std::{ffi::c_void, num::NonZeroIsize, sync::Arc};

use fast_hash_collection::FastHashMap;
use rendiation_viewer_content::*;

mod viewer_api;
pub use viewer_api::*;
mod c_api;
pub use c_api::*;
mod cx;
use cx::*;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ViewerEntityHandle {
  pub index: u32,
  pub generation: u64,
}

impl ViewerEntityHandle {
  pub fn empty() -> Self {
    Self {
      index: u32::MAX,
      generation: u64::MAX,
    }
  }
}
impl<T> From<EntityHandle<T>> for ViewerEntityHandle {
  fn from(value: EntityHandle<T>) -> Self {
    let handle = value.into_raw();
    ViewerEntityHandle {
      index: handle.index(),
      generation: handle.generation(),
    }
  }
}
impl<T> From<ViewerEntityHandle> for EntityHandle<T> {
  fn from(value: ViewerEntityHandle) -> Self {
    unsafe { EntityHandle::from_raw(value.into()) }
  }
}
impl From<RawEntityHandle> for ViewerEntityHandle {
  fn from(value: RawEntityHandle) -> Self {
    ViewerEntityHandle {
      index: value.index(),
      generation: value.generation(),
    }
  }
}
impl From<ViewerEntityHandle> for RawEntityHandle {
  fn from(value: ViewerEntityHandle) -> Self {
    RawEntityHandle::create_only_for_testing_with_gen(value.index as usize, value.generation)
  }
}
