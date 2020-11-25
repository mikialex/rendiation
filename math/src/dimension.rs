use std::marker::PhantomData;

use crate::*;

pub trait DimensionalVec<T, const N: usize> {
  type Type;
}

pub struct Vector<T>(PhantomData<T>);

impl<T> DimensionalVec<T, 2> for Vector<T> {
  type Type = Vec2<T>;
}
impl<T> DimensionalVec<T, 3> for Vector<T> {
  type Type = Vec3<T>;
}
impl<T> DimensionalVec<T, 4> for Vector<T> {
  type Type = Vec4<T>;
}

impl<T, const N: usize> DimensionalVec<T, N> for Vector<T> {
  default type Type = !;
}
