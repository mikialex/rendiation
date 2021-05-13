use std::{marker::PhantomData, ops::*};

use crate::{InnerProductSpace, NormalizedVector, Scalar, VectorSpace};

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Space<T, V, S> {
  value: V,
  type_phantom: PhantomData<T>,
  space_marker: PhantomData<S>,
}

impl<T, V, S> Space<T, V, S> {
  #[inline(always)]
  pub fn wrap(v: V) -> Space<T, V, S> {
    Self {
      value: v,
      type_phantom: PhantomData,
      space_marker: PhantomData,
    }
  }
}

impl<T, V: VectorSpace<T>, S> Add for Space<T, V, S> {
  type Output = Space<T, V, S>;
  #[inline(always)]
  fn add(self, rhs: Self) -> Self::Output {
    Space::wrap(self.value + rhs.value)
  }
}
impl<T, V: VectorSpace<T>, S> Sub for Space<T, V, S> {
  type Output = Space<T, V, S>;
  #[inline(always)]
  fn sub(self, rhs: Self) -> Self::Output {
    Space::wrap(self.value - rhs.value)
  }
}
impl<T, V: VectorSpace<T>, S> Mul<T> for Space<T, V, S> {
  type Output = Space<T, V, S>;
  #[inline(always)]
  fn mul(self, rhs: T) -> Self::Output {
    Space::wrap(self.value * rhs)
  }
}
impl<T, V: VectorSpace<T>, S> Div<T> for Space<T, V, S> {
  type Output = Space<T, V, S>;
  #[inline(always)]
  fn div(self, rhs: T) -> Self::Output {
    Space::wrap(self.value / rhs)
  }
}
impl<T: Copy, V: VectorSpace<T>, S: Copy> VectorSpace<T> for Space<T, V, S> {}
impl<T: Scalar, V: InnerProductSpace<T>, S: Copy> InnerProductSpace<T> for Space<T, V, S> {
  fn dot_impl(&self, b: Self) -> T {
    self.value.dot(b.value)
  }
}

impl<T, V, S> Deref for Space<T, V, S> {
  type Target = V;
  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    &self.value
  }
}
impl<T, V, S> DerefMut for Space<T, V, S> {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.value
  }
}

pub type NormalizedSpace<T, V, S> = NormalizedVector<T, Space<T, V, S>>;

#[derive(Copy, Clone)]
pub struct LocalSpace;
#[derive(Copy, Clone)]
pub struct WorldSpace;

#[test]
fn test() {
  use crate::*;
  let a: NormalizedSpace<f32, Vec3<f32>, LocalSpace> =
    Space::wrap(Vec3::new(1., 1., 1.)).into_normalized();

  let ap: NormalizedSpace<f32, Vec3<f32>, LocalSpace> =
    Space::wrap(Vec3::new(1., 2., 1.)).into_normalized();

  let _ax = a + ap; // only two vector in one space can add together;

  // let _ax = _ax + ap; // todo

  let a = a.normalize(); // should use cheaper method
  let b = Vec3::new(1., 1., 1.);
  let _c = **a + b;
}
