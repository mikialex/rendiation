use std::fmt::Debug;
use std::{fmt, ops::*};

use crate::*;

#[repr(C)]
#[derive(Serialize, Deserialize)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq, Facet)]
pub struct Vec2<T> {
  pub x: T,
  pub y: T,
}

unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Vec2<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Vec2<T> {}

impl<T: Scalar> VectorDimension<2> for Vec2<T> {}
impl<T: Scalar> RealVector<T> for Vec2<T> {}
impl<T> VectorSpace<T> for Vec2<T> where
  T: Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T> + Div<T, Output = T> + Copy
{
}
impl<T: Scalar> InnerProductSpace<T> for Vec2<T> {
  #[inline]
  fn dot_impl(&self, b: Self) -> T {
    self.x * b.x + self.y * b.y
  }
}
impl<T: One + Zero + Copy> Vector<T> for Vec2<T> {
  #[inline]
  fn create<F>(f: F) -> Self
  where
    F: Fn() -> T,
  {
    Self { x: f(), y: f() }
  }

  #[inline]
  fn map<F>(self, f: F) -> Self
  where
    F: Fn(T) -> T,
  {
    Self {
      x: f(self.x),
      y: f(self.y),
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
    }
  }

  fn channel_count() -> usize {
    2
  }
}

impl<T> Vec2<T>
where
  T: Scalar,
{
  #[inline]
  #[must_use]
  pub fn rotate(&self, anchor: Self, radians: T) -> Self {
    let v = *self - anchor;
    let x = v.x;
    let y = v.y;
    let c = radians.cos();
    let s = radians.sin();
    Self {
      x: x * c - y * s,
      y: x * s + y * c,
    }
  }

  #[inline]
  #[must_use]
  pub fn perpendicular_cw(&self) -> Self {
    Self {
      x: self.y,
      y: -self.x,
    }
  }

  #[inline]
  #[must_use]
  pub fn perpendicular_ccw(&self) -> Self {
    Self {
      x: -self.y,
      y: self.x,
    }
  }
}

impl<T> fmt::Display for Vec2<T>
where
  T: Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "({:?}, {:?})", self.x, self.y)
  }
}

impl<T> Vec2<T>
where
  T: Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T> + Copy,
{
  /// Return the cross product of the two vectors.
  #[must_use]
  pub fn cross(self, b: Self) -> T {
    self.x * b.y - self.y * b.x
  }
}

impl<T> BitAnd for Vec2<T>
where
  T: BitAnd<T, Output = T>,
{
  type Output = Self;
  #[inline]
  fn bitand(self, rhs: Self) -> Self::Output {
    Self {
      x: self.x & rhs.x,
      y: self.y & rhs.y,
    }
  }
}
