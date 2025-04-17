use std::ops::{Add, Div, Mul};

use crate::*;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq, Facet)]
#[rustfmt::skip]
pub struct Mat4x3<T> {
  pub a1: T, pub a2: T, pub a3: T,
  pub b1: T, pub b2: T, pub b3: T,
  pub c1: T, pub c2: T, pub c3: T,
  pub d1: T, pub d2: T, pub d3: T,
}

impl<T> Mat4x3<T> {
  pub fn to_mat3(self) -> Mat3<T> {
    #[rustfmt::skip]
    Mat3 {
      a1: self.a1, a2: self.a2, a3: self.a3,
      b1: self.b1, b2: self.b2, b3: self.b3,
      c1: self.c1, c2: self.c2, c3: self.c3,
    }
  }
}

impl<T: Scalar> Mat4x3<T> {
  pub fn to_normal_matrix(self) -> Mat3<T> {
    self.to_mat3().inverse_or_identity().transpose()
  }
}

unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Mat4x3<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Mat4x3<T> {}

impl<T> Mul<Vec3<T>> for Mat4x3<T>
where
  T: Copy + Add<Output = T> + Mul<Output = T> + Div<Output = T> + num_traits::One,
{
  type Output = Vec3<T>;

  fn mul(self, v: Vec3<T>) -> Vec3<T> {
    let v = self * v.expand_with_one();
    Vec3::new(v.x, v.y, v.z) / v.w
  }
}

impl<T> Mul<Vec4<T>> for Mat4x3<T>
where
  T: Copy + Add<Output = T> + Mul<Output = T>,
{
  type Output = Vec4<T>;

  fn mul(self, v: Vec4<T>) -> Vec4<T> {
    Vec4 {
      x: v.x * self.a1 + v.y * self.b1 + v.z * self.c1 + v.w * self.d1,
      y: v.x * self.a2 + v.y * self.b2 + v.z * self.c2 + v.w * self.d2,
      z: v.x * self.a3 + v.y * self.b3 + v.z * self.c3 + v.w * self.d3,
      w: v.w,
    }
  }
}

#[rustfmt::skip]
impl<T: Sized> Mat4x3<T> {
  pub const fn new(
    m11: T, m12: T, m13: T,
    m21: T, m22: T, m23: T,
    m31: T, m32: T, m33: T,
    m41: T, m42: T, m43: T,
  ) -> Self {
    Self {
      a1: m11, a2: m12, a3: m13,
      b1: m21, b2: m22, b3: m23,
      c1: m31, c2: m32, c3: m33,
      d1: m41, d2: m42, d3: m43,
    }
  }
}

impl<T: Copy> Mat4x3<T> {
  pub fn new_from_column(c1: [T; 3], c2: [T; 3], c3: [T; 3], c4: [T; 3]) -> Self {
    #[rustfmt::skip]
    Self {
      a1: c1[0], a2: c1[1], a3: c1[2],
      b1: c2[0], b2: c2[1], b3: c2[2],
      c1: c3[0], c2: c3[1], c3: c3[2],
      d1: c4[0], d2: c4[1], d3: c4[2],
    }
  }
}

impl<T> Mat4x3<T>
where
  T: Scalar,
{
  pub fn from_orth_basis_and_position(forward: Vec3<T>, up: Vec3<T>, position: Vec3<T>) -> Self {
    let right = forward.cross(up);
    #[rustfmt::skip]
    Mat4x3::new(
      right.x,    right.y,    right.z,
      up.x,       up.y,       up.z,
      forward.x,  forward.y,  forward.z,
      position.x, position.y,  position.z,
    )
  }

  pub fn right(&self) -> Vec3<T> {
    Vec3::new(self.a1, self.a2, self.a3)
  }

  pub fn up(&self) -> Vec3<T> {
    Vec3::new(self.b1, self.b2, self.b3)
  }

  pub fn forward(&self) -> Vec3<T> {
    Vec3::new(self.c1, self.c2, self.c3)
  }

  pub fn position(&self) -> Vec3<T> {
    Vec3::new(self.d1, self.d2, self.d3)
  }

  pub fn get_scale(&self) -> Vec3<T> {
    let sx = Vec3::new(self.a1, self.a2, self.a3).length();
    let sy = Vec3::new(self.b1, self.b2, self.b3).length();
    let sz = Vec3::new(self.c1, self.c2, self.c3).length();
    Vec3::new(sx, sy, sz)
  }
}

impl<T: Scalar> From<Mat4<T>> for Mat4x3<T> {
  fn from(m: Mat4<T>) -> Self {
    assert_eq!(m.a4, T::zero());
    assert_eq!(m.b4, T::zero());
    assert_eq!(m.c4, T::zero());
    assert_eq!(m.d4, T::one());
    #[rustfmt::skip]
    Self {
      a1: m.a1,      a2: m.a2,      a3: m.a3,
      b1: m.b1,      b2: m.b2,      b3: m.b3,
      c1: m.c1,      c2: m.c2,      c3: m.c3,
      d1: T::zero(), d2: T::zero(), d3: T::zero(),
    }
  }
}

impl<T: Scalar> From<Mat3<T>> for Mat4x3<T> {
  fn from(m: Mat3<T>) -> Self {
    #[rustfmt::skip]
    Self {
      a1: m.a1,      a2: m.a2,      a3: m.a3,
      b1: m.b1,      b2: m.b2,      b3: m.b3,
      c1: m.c1,      c2: m.c2,      c3: m.c3,
      d1: T::zero(), d2: T::zero(), d3: T::zero(),
    }
  }
}

impl<T> AsRef<Mat4x3<T>> for Mat4x3<T> {
  fn as_ref(&self) -> &Mat4x3<T> {
    self
  }
}

impl<T> AsMut<Mat4x3<T>> for Mat4x3<T> {
  fn as_mut(&mut self) -> &mut Mat4x3<T> {
    self
  }
}
