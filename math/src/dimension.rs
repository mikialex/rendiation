use std::{
  fmt::Debug,
  marker::PhantomData,
  ops::Deref,
  ops::{Add, DerefMut},
};

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

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Vector<T, const N: usize> {
  pub data: <VectorMark<T> as DimensionalVec<T, N>>::Type,
}

impl<T, const N: usize> Deref for Vector<T, N> {
  default type Target = <VectorMark<T> as DimensionalVec<T, N>>::Type;

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

impl<T, const N: usize> Add for Vector<T, N>
where
  <VectorMark<T> as DimensionalVec<T, N>>::Type:
    Add<Output = <VectorMark<T> as DimensionalVec<T, N>>::Type>,
{
  type Output = Vector<T, N>;

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
