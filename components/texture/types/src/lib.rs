#![feature(nonzero_is_power_of_two)]
#![feature(int_log)]

use std::num::NonZeroUsize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size {
  pub width: NonZeroUsize,
  pub height: NonZeroUsize,
}

impl Size {
  pub fn max_side_length(&self) -> NonZeroUsize {
    self.width.max(self.height)
  }

  /// return value is all mipmap levels plus base level(1)
  pub fn mip_level_count(&self) -> usize {
    let len: usize = self.max_side_length().into();
    len.next_power_of_two().log2() + 1
  }

  pub fn is_pot(&self) -> bool {
    self.width.is_power_of_two() && self.height.is_power_of_two()
  }

  pub fn from_u32_pair_min_one(size: (u32, u32)) -> Self {
    let width = NonZeroUsize::new(size.0 as usize).unwrap_or(NonZeroUsize::new(1).unwrap());
    let height = NonZeroUsize::new(size.1 as usize).unwrap_or(NonZeroUsize::new(1).unwrap());
    Size { width, height }
  }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CubeTextureFace {
  PositiveX = 0,
  NegativeX = 1,
  PositiveY = 2,
  NegativeY = 3,
  PositiveZ = 4,
  NegativeZ = 5,
}
