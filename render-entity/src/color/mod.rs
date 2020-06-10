pub mod color_space;
pub mod rgb;
pub mod hsl;

pub use color_space::*;
pub use rgb::*;
pub use hsl::*;

use std::{ops::Mul};

#[derive(Debug)]
pub struct Color<T: ColorSpace = SRGBColorSpace<f32>> {
  value: T::ContainerValue,
}

// why i cant derive ??
impl<T: ColorSpace> Clone for Color<T> {
  fn clone(&self) -> Self {
      *self
  }
}
impl<T: ColorSpace> Copy for Color<T> { }

// multiply scalar
impl<T: ColorSpace, U> Mul<U> for Color<T>
where
  T::ContainerValue: Mul<U, Output = T::ContainerValue> + Copy,
{
  type Output = Self;

  fn mul(self, other: U) -> Self {
    Self {
      value: self.value * other,
    }
  }
}

impl<T: ColorSpace> Color<T> {
  pub fn new(value: T::ContainerValue) -> Self {
    Self { value }
  }
  pub fn from_value(value: impl Into<T::ContainerValue>) -> Self {
    Self { value: value.into() }
  }
}
