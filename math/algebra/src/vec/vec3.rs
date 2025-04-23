use std::fmt::Debug;
use std::{fmt, ops::*};

use crate::*;

#[repr(C)]
#[derive(Serialize, Deserialize)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq, Facet)]
pub struct Vec3<T> {
  pub x: T,
  pub y: T,
  pub z: T,
}

unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Vec3<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Vec3<T> {}

impl<T: Scalar> VectorDimension<3> for Vec3<T> {}
impl<T: Scalar> RealVector<T> for Vec3<T> {}
impl<T> VectorSpace<T> for Vec3<T> where
  T: Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T> + Div<T, Output = T> + Copy
{
}
impl<T: Scalar> InnerProductSpace<T> for Vec3<T> {
  #[inline]
  fn dot_impl(&self, b: Self) -> T {
    self.x * b.x + self.y * b.y + self.z * b.z
  }
}
impl<T: One + Zero + Copy> Vector<T> for Vec3<T> {
  #[inline]
  fn create<F>(f: F) -> Self
  where
    F: Fn() -> T,
  {
    Self {
      x: f(),
      y: f(),
      z: f(),
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
    }
  }
  fn channel_count() -> usize {
    3
  }
}

impl<T> Vec3<T>
where
  T: Scalar,
{
  /// input: Matrix4 affine matrix
  ///
  /// vector interpreted as a direction
  #[inline]
  #[must_use]
  pub fn transform_direction(&self, m: Mat4<T>) -> NormalizedVector<T, Self> {
    Self {
      x: m.a1 * self.x + m.b1 * self.y + m.c1 * self.z,
      y: m.a2 * self.x + m.b2 * self.y + m.c2 * self.z,
      z: m.a3 * self.x + m.b3 * self.y + m.c3 * self.z,
    }
    .into_normalized()
  }

  #[inline]
  pub fn max_channel(self) -> T {
    self.x.max(self.y).max(self.z)
  }
  #[inline]
  pub fn min_channel(self) -> T {
    self.x.min(self.y).min(self.z)
  }
}

impl<T> Vec3<T>
where
  T: Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T> + Copy,
{
  /// Return the cross product of the two vectors.
  #[must_use]
  pub fn cross(self, b: Self) -> Self {
    Self {
      x: self.y * b.z - self.z * b.y,
      y: self.z * b.x - self.x * b.z,
      z: self.x * b.y - self.y * b.x,
    }
  }
}

impl<T: Scalar> NormalizedVector<T, Vec3<T>> {
  pub fn local_to_world(&self) -> Mat3<T> {
    let v = self.value;
    let ns = if v.x.is_normal() {
      Vec3::new(v.y, -v.x, T::zero()).normalize()
    } else {
      Vec3::new(T::zero(), -v.z, v.y).normalize()
    };
    let nss = v.cross(ns);
    let ns = v.cross(nss);

    #[rustfmt::skip]
    Mat3::new(
      nss.x, nss.y, nss.z,
       ns.x,  ns.y,  ns.z,
        v.x,   v.y,   v.z
    )
  }
}

impl<T> fmt::Display for Vec3<T>
where
  T: Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "({:?}, {:?}, {:?})", self.x, self.y, self.z)
  }
}

impl<T> Vec3<T> {
  #[must_use]
  pub fn expand_with(self, channel: T) -> Vec4<T> {
    Vec4::new(self.x, self.y, self.z, channel)
  }
  #[must_use]
  pub fn expand_with_one(self) -> Vec4<T>
  where
    T: One,
  {
    Vec4::new(self.x, self.y, self.z, T::one())
  }
}
