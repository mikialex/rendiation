use crate::*;
use std::fmt::Debug;
use std::{fmt, ops::*};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Vec3<T> {
  pub x: T,
  pub y: T,
  pub z: T,
}

unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Vec3<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Vec3<T> {}

impl<T: Scalar> VectorDimension<3> for Vec3<T> {}
impl<T: Scalar> VectorImpl for Vec3<T> {}
impl<T: Scalar> RealVector<T> for Vec3<T> {}
impl<T> VectorSpace<T> for Vec3<T> where
  T: Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T> + Copy
{
}
impl<T: Scalar> InnerProductSpace<T> for Vec3<T> {
  #[inline]
  fn dot(&self, b: Self) -> T {
    self.x * b.x + self.y * b.y + self.z * b.z
  }
}
impl<T: One + Zero + Copy + Sub<T, Output = T>> Vector<T> for Vec3<T> {
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
}

impl<T> Vec3<T>
where
  T: Scalar,
{
  /// input: Matrix4 affine matrix
  ///
  /// vector interpreted as a direction
  #[inline]
  pub fn transform_direction(&self, m: Mat4<T>) -> Self {
    Self {
      x: m.a1 * self.x + m.b1 * self.y + m.c1 * self.z,
      y: m.a2 * self.x + m.b2 * self.y + m.c2 * self.z,
      z: m.a3 * self.x + m.b3 * self.y + m.c3 * self.z,
    }
    .normalize()
  }

  #[inline]
  pub fn max_channel(self) -> T {
    self.x.max(self.y).max(self.z)
  }
}

impl<T> Vec3<T>
where
  T: Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T> + Copy,
{
  /// Return the cross product of the two vectors.
  pub fn cross(self, b: Self) -> Self {
    Self {
      x: self.y * b.z - self.z * b.y,
      y: self.z * b.x - self.x * b.z,
      z: self.x * b.y - self.y * b.x,
    }
  }
}

impl Vec3<f32> {
  /// self should be normalized
  pub fn local_to_world(&self) -> Mat3<f32> {
    let ns = if self.x.is_normal() {
      Vec3::new(self.y, -self.x, 0.0).normalize()
    } else {
      Vec3::new(0.0, -self.z, self.y).normalize()
    };
    let nss = self.cross(ns);
    Mat3::new(
      ns.x, nss.x, self.x, ns.y, nss.y, self.y, ns.z, nss.z, self.z,
    )
  }
}

impl<T> Vec3<T> {
  pub fn set(&mut self, x: T, y: T, z: T) -> &Self {
    self.x = x;
    self.y = y;
    self.z = z;
    self
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

// impl Vec3<f32> {
//   pub fn apply_mat4(&self, mat: &Mat4<f32>) -> Self {
//     let w = 1. / (mat.a4 * self.x + mat.b4 * self.y + mat.c4 * self.z + mat.d4);

//     Self {
//       x: (mat.a1 * self.x + mat.b1 * self.y + mat.c1 * self.z + mat.d1) * w,
//       y: (mat.a2 * self.x + mat.b2 * self.y + mat.c2 * self.z + mat.d2) * w,
//       z: (mat.a3 * self.x + mat.b3 * self.y + mat.c3 * self.z + mat.d3) * w,
//     }
//   }
// }
