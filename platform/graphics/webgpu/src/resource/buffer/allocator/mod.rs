use crate::*;

mod slab;
pub use slab::*;

mod range;
pub use range::*;

pub struct RelocationMessage {
  pub previous_offset: u32,
  pub new_offset: u32,
}

pub trait AllocatorStorageBase: LinearStorageBase {
  fn current_used(&self) -> u32;
  /// return if reserve_success
  ///  
  /// note that reserve success not necessary means range allocate
  /// will always success because of fragmentation. This methods used is only for performance consideration.
  fn try_reserve_used(
    &mut self,
    _used: u32,
    _relocation_handler: &mut dyn FnMut(RelocationMessage),
  ) {
    // empty default impl
  }

  fn try_compact(&mut self, _relocation_handler: &mut dyn FnMut(RelocationMessage)) {
    // empty default impl
  }
}

pub trait LinearAllocatorStorage: AllocatorStorageBase {
  fn deallocate(&mut self, idx: u32);
  fn allocate_value(&mut self, v: Self::Item) -> Option<u32>;
}

pub trait RangeAllocatorStorage: AllocatorStorageBase {
  fn deallocate(&mut self, idx: u32);
  fn allocate_values(
    &mut self,
    v: &[Self::Item],
    relocation_handler: &mut dyn FnMut(RelocationMessage),
  ) -> Option<u32>;

  /// LinearStorageDirectAccess bound is required or this method will be useless
  fn allocate_range(
    &mut self,
    count: u32,
    relocation_handler: &mut dyn FnMut(RelocationMessage),
  ) -> Option<u32>
  where
    Self: LinearStorageDirectAccess;
}
