use crate::*;
use std::ops::Deref;

#[derive(Debug, Copy, Clone)]
pub struct Normalized<T: Vector>(T);

impl<T: Vector> Normalized<T> {
  pub fn value(&self) -> T {
    self.0
  }

  pub fn into_normalized(inner: T) -> Self {
    Self(inner.normalize())
  }

  pub fn normalize(&self) -> Self {
    *self // normalized is normalized
  }

  pub unsafe fn as_normalized(inner: T) -> Self {
    Self(inner)
  }
}

impl<T: Vector> Deref for Normalized<T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
