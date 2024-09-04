use crate::*;

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
  fn try_reserve_used(&mut self, used: u32, relocation_handler: &mut dyn FnMut(RelocationMessage));

  fn try_compact(&mut self, relocation_handler: &mut dyn FnMut(RelocationMessage));
}

pub trait LinearAllocatorStorage: AllocatorStorageBase {
  fn remove(&mut self, idx: u32);
  fn set_value(&mut self, v: Self::Item) -> Option<usize>;
}

pub trait RangeAllocatorStorage: AllocatorStorageBase {
  fn remove(&mut self, idx: u32);
  fn set_values(
    &mut self,
    v: &[Self::Item],
    relocation_handler: &mut dyn FnMut(RelocationMessage),
  ) -> Option<u32>;
}
