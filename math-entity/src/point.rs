use std::ops::Deref;

use rendiation_math::VectorType;

use crate::SpaceEntity;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point<T>(pub T);

impl<T: Copy> Point<T> {
  pub fn new(v: T) -> Self {
    Self(v)
  }
}

impl<T> Deref for Point<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<V: AsRef<VectorType<f32, D>>, const D: usize> SpaceEntity<D> for Point<V> {}
