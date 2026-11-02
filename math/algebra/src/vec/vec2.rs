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
  /// Rotate the point counter clockwise around the `anchor` point.
  #[inline]
  #[must_use]
  pub fn rotate(&self, anchor: Self, radians: T) -> Self {
    let v = *self - anchor;
    let (s, c) = radians.sin_cos();
    anchor
      + Self {
        x: v.x * c - v.y * s,
        y: v.x * s + v.y * c,
      }
  }

  /// Rotate 90 degrees clockwise.
  #[inline]
  #[must_use]
  pub fn perpendicular_cw(&self) -> Self {
    Self {
      x: self.y,
      y: -self.x,
    }
  }

  /// Rotate 90 degrees counter clockwise.
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
  /// Return the 2d cross product (the z component of the 3d cross product), which is positive
  /// if `b` is counter clockwise of self.
  #[must_use]
  pub fn cross(self, b: Self) -> T {
    self.x * b.y - self.y * b.x
  }
}
