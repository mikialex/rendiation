use std::{
  marker::PhantomData,
  ops::Deref,
  ops::{Add, DerefMut},
};

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

#[repr(transparent)]
pub struct Vector<T, const D: usize> {
  pub data: <VectorMark<T> as DimensionalVec<T, D>>::Type,
}

impl<T, const D: usize> Copy for Vector<T, D> where
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Copy
{
}

impl<T, const D: usize> Clone for Vector<T, D>
where
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Clone,
{
  fn clone(&self) -> Self {
    Self {
      data: self.data.clone(),
    }
  }
}

impl<T, const D: usize> Deref for Vector<T, D> {
  default type Target = <VectorMark<T> as DimensionalVec<T, D>>::Type;

  default fn deref(&self) -> &Self::Target {
    unreachable!()
  }
}

impl<T> Deref for Vector<T, 2> {
  type Target = Vec2<T>;

  fn deref(&self) -> &Self::Target {
    unsafe { std::mem::transmute(&self.data) }
  }
}
impl<T> DerefMut for Vector<T, 2> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { std::mem::transmute(&mut self.data) }
  }
}

impl<T, const D: usize> Add for Vector<T, D>
where
  <VectorMark<T> as DimensionalVec<T, D>>::Type:
    Add<Output = <VectorMark<T> as DimensionalVec<T, D>>::Type>,
{
  type Output = Vector<T, D>;

  fn add(self, rhs: Self) -> Self::Output {
    Self {
      data: self.data + rhs.data,
    }
  }
}

#[test]
fn test() {
  let a: Vector<f32, 2> = Vector {
    data: Vec2::new(1., 0.),
  };
  let b: Vector<f32, 2> = Vector {
    data: Vec2::new(1., 1.),
  };
  let x = a.length();
  assert_eq!(1., x);
  let y: Vector<f32, 2> = a + b;
}
