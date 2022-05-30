use crate::*;
use std::{fmt, ops::Sub};
use std::{fmt::Debug, ops::*};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Vec4<T> {
  pub x: T,
  pub y: T,
  pub z: T,
  pub w: T,
}

pub fn vec4<T>(x: T, y: T, z: T, w: T) -> Vec4<T> {
  Vec4::new(x, y, z, w)
}

unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Vec4<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Vec4<T> {}

impl<T: Scalar> VectorDimension<4> for Vec4<T> {}
impl<T: Scalar> VectorSelfCrateImpl for Vec4<T> {}
impl<T: Scalar> RealVector<T> for Vec4<T> {}
impl<T> VectorSpace<T> for Vec4<T> where
  T: Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T> + Div<T, Output = T> + Copy
{
}
impl<T: Scalar> InnerProductSpace<T> for Vec4<T> {
  #[inline]
  fn dot_impl(&self, b: Self) -> T {
    self.x * b.x + self.y * b.y + self.z * b.z + self.w * b.w
  }
}
impl<T: One + Zero + Copy + Sub<T, Output = T>> Vector<T> for Vec4<T> {
  #[inline]
  fn create<F>(f: F) -> Self
  where
    F: Fn() -> T,
  {
    Self {
      x: f(),
      y: f(),
      z: f(),
      w: f(),
    }
  }

  #[inline]
  fn map<F>(self, f: F) -> Self
  where
    F: Fn(T) -> T,
  {
    Self {
      x: f(self.x),
      y: f(self.y),
      z: f(self.z),
      w: f(self.w),
    }
  }

  #[inline]
  fn zip<F>(self, v2: Self, f: F) -> Self
  where
    F: Fn(T, T) -> T,
  {
    Self {
      x: f(self.x, v2.x),
      y: f(self.y, v2.y),
      z: f(self.z, v2.z),
      w: f(self.w, v2.w),
    }
  }
}

impl<T> fmt::Display for Vec4<T>
where
  T: Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "({:?}, {:?}, {:?}, {:?})",
      self.x, self.y, self.z, self.w
    )
  }
}
