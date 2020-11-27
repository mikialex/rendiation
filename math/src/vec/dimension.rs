use std::{marker::PhantomData, ops::Mul};

use crate::*;

pub trait Vector<T: Scalar>: Sized + Mul<T, Output = Self> + Copy {
  #[inline]
  fn normalize(&self) -> Self {
    let mag_sq = self.length2();
    if mag_sq > T::zero() {
      let inv_sqrt = T::one() / mag_sq.sqrt();
      return *self * inv_sqrt;
    }
    *self
  }

  #[inline]
  fn length2(&self) -> T {
    self.dot(*self)
  }

  fn dot(&self, b: Self) -> T;
}

pub trait DimensionalVec<T: Scalar, const D: usize> {
  type Type: Vector<T>;
}

pub struct VectorMark<T>(PhantomData<T>);

impl<T: Scalar> DimensionalVec<T, 2> for VectorMark<T> {
  type Type = Vec2<T>;
}
impl<T: Scalar> DimensionalVec<T, 3> for VectorMark<T> {
  type Type = Vec3<T>;
}
impl<T: Scalar> DimensionalVec<T, 4> for VectorMark<T> {
  type Type = Vec4<T>;
}

impl<T: Scalar, const D: usize> DimensionalVec<T, D> for VectorMark<T> {
  default type Type = Vec2<T>; // todo impl for [T; D]
}
