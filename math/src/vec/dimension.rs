use std::marker::PhantomData;

use crate::*;

pub trait Vector {
  fn normalize(&self) -> Self;
}

pub trait DimensionalVec<T, const D: usize> {
  type Type: Vector;
}

pub struct VectorMark<T>(PhantomData<T>);

impl<T: Copy> DimensionalVec<T, 2> for VectorMark<T> {
  type Type = Vec2<T>;
}
impl<T: Copy> DimensionalVec<T, 3> for VectorMark<T> {
  type Type = Vec3<T>;
}
impl<T: Copy> DimensionalVec<T, 4> for VectorMark<T> {
  type Type = Vec4<T>;
}

impl<T: Copy, const D: usize> DimensionalVec<T, D> for VectorMark<T> {
  default type Type = [T; D];
}
