pub mod hsl;
pub mod rgb;

pub use hsl::*;
pub use rgb::*;

use std::ops::Mul;

#[derive(Debug)]
#[repr(transparent)]
pub struct Color<S = f32, T = SRGBColorSpace<f32>>
where
  T: ColorSpace<S>,
{
  pub value: T::Value,
}

pub trait ColorSpace<S> {
  type Value: Copy + Clone;
}

pub trait RGBColorSpace<T>: ColorSpace<T> {}
pub trait HSLColorSpace<T>: ColorSpace<T> {}

impl<S, T: ColorSpace<S>> Clone for Color<S, T> {
  fn clone(&self) -> Self {
    *self
  }
}
impl<S, T: ColorSpace<S>> Copy for Color<S, T> {}

// multiply scalar
impl<S, T: ColorSpace<S>, U> Mul<U> for Color<S, T>
where
  T::Value: Mul<U, Output = T::Value> + Copy,
{
  type Output = Self;

  fn mul(self, other: U) -> Self {
    Self {
      value: self.value * other,
    }
  }
}

impl<S, T: ColorSpace<S>> Color<S, T> {
  pub fn new(value: T::Value) -> Self {
    Self { value }
  }
  pub fn from_value(value: impl Into<T::Value>) -> Self {
    Self {
      value: value.into(),
    }
  }
}
