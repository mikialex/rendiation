#![allow(clippy::many_single_char_names)]

pub mod hsl;
pub mod rgb;
pub mod ycocg;

use std::ops::{Deref, DerefMut, Mul};

pub use hsl::*;
pub use rgb::*;
pub use ycocg::*;

#[derive(Debug, Clone, Copy)]
pub struct ColorWithAlpha<C, T> {
  pub color: C,
  pub a: T,
}

impl<C, T> ColorWithAlpha<C, T> {
  pub fn new(color: C, a: T) -> Self {
    Self { color, a }
  }
}

/// Only RGB Color has meaningful premultiplied alpha
pub trait RGBColor<T>: Mul<T, Output = Self> + Copy {}

impl<C: RGBColor<T>, T: Copy> ColorWithAlpha<C, T> {
  pub fn get_premultiplied(&self) -> C {
    self.color * self.a
  }
}

impl<C: From<(T, T, T)>, T> From<(T, T, T, T)> for ColorWithAlpha<C, T> {
  fn from(value: (T, T, T, T)) -> Self {
    Self::new((value.0, value.1, value.2).into(), value.3)
  }
}

impl<C: Into<(T, T, T)>, T> From<ColorWithAlpha<C, T>> for (T, T, T, T) {
  fn from(r: ColorWithAlpha<C, T>) -> Self {
    let value: (T, T, T) = r.color.into();
    (value.0, value.1, value.2, r.a)
  }
}

impl<C, T> Deref for ColorWithAlpha<C, T> {
  type Target = C;

  fn deref(&self) -> &Self::Target {
    &self.color
  }
}

impl<C, T> DerefMut for ColorWithAlpha<C, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.color
  }
}
