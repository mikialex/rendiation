use crate::*;

pub trait DeviceRadixSortKey {
  const MAX_BITS: u32;
  fn is_one(value: Node<Self>, bit_position: Node<u32>) -> Node<bool>;
}

impl DeviceRadixSortKey for u32 {
  const MAX_BITS: u32 = 32;
  fn is_one(value: Node<Self>, bit_position: Node<u32>) -> Node<bool> {
    (value & (val(1) << bit_position)).not_equals(val(0))
  }
}
