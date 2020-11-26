use crate::*;
use std::ops::Deref;

#[derive(Debug, Copy, Clone)]
pub struct Normalized<T: VectorTrait>(T);

impl<T: VectorTrait> Normalized<T> {
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

impl<T: VectorTrait> Deref for Normalized<T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
