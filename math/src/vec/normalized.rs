use std::{marker::PhantomData, ops::*};

use crate::*;

pub trait InnerData<T>: Copy {
  fn get_inner(self) -> T;
}

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct NormalizedVector<T, V> {
  value: V,
  phantom: PhantomData<T>,
}

impl<T, V> NormalizedVector<T, V> {
  pub fn wrap(v: V) -> Self {
    Self {
      value: v,
      phantom: PhantomData,
    }
  }
}

impl<T: Scalar, V: InnerProductSpace<T> + Vector<T>> NormalizedVector<T, V> {
  #[inline]
  pub fn normalize(&self) -> Self {
    *self
  }

  /// normalized vector reflect should be normalized
  ///
  /// and input normal should also be normalized
  #[inline]
  pub fn reflect(&self, normal: Self) -> Self {
    NormalizedVector::wrap(self.value.reflect(*normal))
  }

  #[inline]
  pub fn length(&self) -> T {
    T::one()
  }

  #[inline]
  pub fn length2(&self) -> T {
    T::one()
  }
}

// after add / sub, the vector may not be normalized
impl<T, V: VectorSpace<T>> Add for NormalizedVector<T, V> {
  type Output = V;
  fn add(self, rhs: Self) -> Self::Output {
    self.value + rhs.value
  }
}
impl<T, V: VectorSpace<T>> Sub for NormalizedVector<T, V> {
  type Output = V;
  fn sub(self, rhs: Self) -> Self::Output {
    self.value - rhs.value
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
