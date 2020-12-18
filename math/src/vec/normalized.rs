use crate::*;
use std::{
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Normalized<T>(T);

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

impl<T> Deref for Normalized<T> {
  type Target = T;
  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
impl<T> DerefMut for Normalized<T> {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

#[test]
fn test() {
  let a = Normalized(Vec3::new(1., 1., 1.));
  let b = Vec3::new(1., 1., 1.);
  let _c = *a + b;
  a.normalize();
}
