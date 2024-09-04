use crate::*;

mod range;
pub use range::*;

pub trait AllocatorStorageBase: LinearStorageBase {
  fn current_used(&self) -> u32;
  /// return if reserve_success
  fn reserve_used(&mut self, used: u32, relocation_handler: &mut dyn FnMut((u32, u32))) -> bool;
  fn try_compact(&mut self, relocation_handler: &mut dyn FnMut((u32, u32)));
}

pub trait LinearAllocatorStorage: AllocatorStorageBase {
  fn remove(&mut self, idx: u32);
  fn set_value(&mut self, v: Self::Item) -> Option<usize>;
}

pub trait RangeAllocatorStorage: AllocatorStorageBase {
  fn remove(&mut self, idx: u32);
  fn set_values(&mut self, v: &[Self::Item]) -> Option<usize>;
}
