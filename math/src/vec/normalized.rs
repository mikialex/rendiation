use std::{
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

use crate::*;

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct NormalizedVector<T: Scalar, V: Vector<T>> {
  value: V,
  phantom: PhantomData<T>,
}

impl<T: Scalar, V: Vector<T>> NormalizedVector<T, V> {
  pub fn wrap(v: V) -> Self {
    Self {
      value: v,
      phantom: PhantomData,
    }
  }
}

impl<T: Scalar, V: Vector<T>> NormalizedVector<T, V> {
  pub fn normalize(&self) -> Self {
    println!("skip");
    *self
  }
}

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Space<T, S> {
  value: T,
  space_marker: PhantomData<S>,
}

impl<T, S> Deref for Space<T, S> {
  type Target = T;
  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    &self.value
  }
}
impl<T, S> DerefMut for Space<T, S> {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.value
  }
}

impl<T: Scalar, V: Vector<T>> Deref for NormalizedVector<T, V> {
  type Target = V;
  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    &self.value
  }
}
impl<T: Scalar, V: Vector<T>> DerefMut for NormalizedVector<T, V> {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.value
  }
}

#[test]
fn test() {
  use crate::*;
  let a = NormalizedVector::wrap(Vec3::new(1., 1., 1.));
  let b = Vec3::new(1., 1., 1.);
  let _c = *a + b;
}
