use std::marker::PhantomData;

use crate::*;

pub trait DimensionalVec<T, const D: usize> {
  type Type;
}

pub struct VectorMark<T>(PhantomData<T>);

impl<T> DimensionalVec<T, 2> for VectorMark<T> {
  type Type = Vec2<T>;
}
impl<T> DimensionalVec<T, 3> for VectorMark<T> {
  type Type = Vec3<T>;
}
impl<T> DimensionalVec<T, 4> for VectorMark<T> {
  type Type = Vec4<T>;
}

impl<T, const D: usize> DimensionalVec<T, D> for VectorMark<T> {
  default type Type = [T; D];
}
