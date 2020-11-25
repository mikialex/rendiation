use std::{fmt::Debug, marker::PhantomData};

use crate::*;

pub trait DimensionalVec<T, const D: usize> {
  type Type: Copy + Debug;
}

pub struct VectorMark<T>(PhantomData<T>);

impl<T: Copy + Debug> DimensionalVec<T, 2> for VectorMark<T> {
  type Type = Vec2<T>;
}
impl<T: Copy + Debug> DimensionalVec<T, 3> for VectorMark<T> {
  type Type = Vec3<T>;
}
impl<T: Copy + Debug> DimensionalVec<T, 4> for VectorMark<T> {
  type Type = Vec4<T>;
}

impl<T, const N: usize> DimensionalVec<T, N> for VectorMark<T> {
  default type Type = !;
}
