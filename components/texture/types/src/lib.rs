#![feature(int_log)]

use std::{num::NonZeroUsize, ops::Mul};

/// Represent a none zero size(width/height)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Size {
  pub width: NonZeroUsize,
  pub height: NonZeroUsize,
}

impl Mul<usize> for Size {
  type Output = Self;

  fn mul(self, rhs: usize) -> Self::Output {
    let new_width = usize::from(self.width) * rhs;
    let new_height = usize::from(self.height) * rhs;
    Self::from_usize_pair_min_one((new_width, new_height))
  }
}

impl Size {
  pub fn area(&self) -> usize {
    usize::from(self.width) * usize::from(self.height)
  }

  pub fn max_side_length(&self) -> NonZeroUsize {
    self.width.max(self.height)
  }

  /// return value is all mipmap levels plus base level(1)
  pub fn mip_level_count(&self) -> usize {
    let len: usize = self.max_side_length().into();
    len.next_power_of_two().log2() as usize + 1
  }

  pub fn is_pot(&self) -> bool {
    self.width.is_power_of_two() && self.height.is_power_of_two()
  }

  #[allow(clippy::or_fun_call)]
  pub fn from_u32_pair_min_one(size: (u32, u32)) -> Self {
    let width = NonZeroUsize::new(size.0 as usize).unwrap_or(NonZeroUsize::new(1).unwrap());
    let height = NonZeroUsize::new(size.1 as usize).unwrap_or(NonZeroUsize::new(1).unwrap());
    Size { width, height }
  }

  #[allow(clippy::or_fun_call)]
  pub fn from_usize_pair_min_one(size: (usize, usize)) -> Self {
    let width = NonZeroUsize::new(size.0).unwrap_or(NonZeroUsize::new(1).unwrap());
    let height = NonZeroUsize::new(size.1).unwrap_or(NonZeroUsize::new(1).unwrap());
    Size { width, height }
  }

  pub fn into_usize(&self) -> (usize, usize) {
    (usize::from(self.width), usize::from(self.height))
  }

  pub fn width_usize(&self) -> usize {
    usize::from(self.width)
  }

  pub fn height_usize(&self) -> usize {
    usize::from(self.height)
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

/// Represent a position in texture2d
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TextureOrigin {
  pub x: usize,
  pub y: usize,
}

impl TextureOrigin {
  pub fn zero() -> Self {
    Self { x: 0, y: 0 }
  }
}

impl From<(usize, usize)> for TextureOrigin {
  fn from(v: (usize, usize)) -> Self {
    Self { x: v.0, y: v.1 }
  }
}

/// Represent a none zero size(width/height) area
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TextureRange {
  pub origin: TextureOrigin,
  pub size: Size,
}
