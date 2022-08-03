use crate::*;
use std::ops::{Add, Div, Mul};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
#[rustfmt::skip]
pub struct Mat4<T> {
  pub a1: T, pub a2: T, pub a3: T, pub a4: T,
  pub b1: T, pub b2: T, pub b3: T, pub b4: T,
  pub c1: T, pub c2: T, pub c3: T, pub c4: T,
  pub d1: T, pub d2: T, pub d3: T, pub d4: T,
}

impl<T: Scalar> SquareMatrixDimension<3> for Mat4<T> {}
impl<T: Scalar> SquareMatrix<T> for Mat4<T> {
  fn identity() -> Self {
    Self::one()
  }
  fn transpose(&self) -> Mat4<T> {
    #[rustfmt::skip]
    Mat4::new(
      self.a1, self.b1, self.c1, self.d1,
      self.a2, self.b2, self.c2, self.d2,
      self.a3, self.b3, self.c3, self.d3,
      self.a4, self.b4, self.c4, self.d4,
    )
  }

  #[rustfmt::skip]
  fn det(&self) -> T {
    let m = self;

      m.a1 * m.b2 * m.c3 * m.d4
    - m.a1 * m.b2 * m.c4 * m.d3 
    + m.a1 * m.b3 * m.c4 * m.d2
    - m.a1 * m.b3 * m.c2 * m.d4
    + m.a1 * m.b4 * m.c2 * m.d3
    - m.a1 * m.b4 * m.c3 * m.d2

    - m.a2 * m.b3 * m.c4 * m.d1
    + m.a2 * m.b3 * m.c1 * m.d4
    - m.a2 * m.b4 * m.c1 * m.d3
    + m.a2 * m.b4 * m.c3 * m.d1
    - m.a2 * m.b1 * m.c3 * m.d4
    + m.a2 * m.b1 * m.c4 * m.d3

    + m.a3 * m.b4 * m.c1 * m.d2
    - m.a3 * m.b4 * m.c2 * m.d1
    + m.a3 * m.b1 * m.c2 * m.d4
    - m.a3 * m.b1 * m.c4 * m.d2
    + m.a3 * m.b2 * m.c4 * m.d1
    - m.a3 * m.b2 * m.c1 * m.d4

    - m.a4 * m.b1 * m.c2 * m.d3
    + m.a4 * m.b1 * m.c3 * m.d2
    - m.a4 * m.b2 * m.c3 * m.d1
    + m.a4 * m.b2 * m.c1 * m.d3
    - m.a4 * m.b3 * m.c1 * m.d2
    + m.a4 * m.b3 * m.c2 * m.d1
  }

  fn inverse(&self) -> Option<Self> {
    let det = self.det();
    if det == T::zero() {
      return None;
    }

    let m = self;
    let inv_det = T::one() / det;

    Self {
      a1:  inv_det * (m.b2 * (m.c3 * m.d4 - m.c4 * m.d3) + m.b3 * (m.c4 * m.d2 - m.c2 * m.d4) + m.b4 * (m.c2 * m.d3 - m.c3 * m.d2)),
      a2: -inv_det * (m.a2 * (m.c3 * m.d4 - m.c4 * m.d3) + m.a3 * (m.c4 * m.d2 - m.c2 * m.d4) + m.a4 * (m.c2 * m.d3 - m.c3 * m.d2)),
      a3:  inv_det * (m.a2 * (m.b3 * m.d4 - m.b4 * m.d3) + m.a3 * (m.b4 * m.d2 - m.b2 * m.d4) + m.a4 * (m.b2 * m.d3 - m.b3 * m.d2)),
      a4: -inv_det * (m.a2 * (m.b3 * m.c4 - m.b4 * m.c3) + m.a3 * (m.b4 * m.c2 - m.b2 * m.c4) + m.a4 * (m.b2 * m.c3 - m.b3 * m.c2)),

      b1: -inv_det * (m.b1 * (m.c3 * m.d4 - m.c4 * m.d3) + m.b3 * (m.c4 * m.d1 - m.c1 * m.d4) + m.b4 * (m.c1 * m.d3 - m.c3 * m.d1)),
      b2:  inv_det * (m.a1 * (m.c3 * m.d4 - m.c4 * m.d3) + m.a3 * (m.c4 * m.d1 - m.c1 * m.d4) + m.a4 * (m.c1 * m.d3 - m.c3 * m.d1)),
      b3: -inv_det * (m.a1 * (m.b3 * m.d4 - m.b4 * m.d3) + m.a3 * (m.b4 * m.d1 - m.b1 * m.d4) + m.a4 * (m.b1 * m.d3 - m.b3 * m.d1)),
      b4:  inv_det * (m.a1 * (m.b3 * m.c4 - m.b4 * m.c3) + m.a3 * (m.b4 * m.c1 - m.b1 * m.c4) + m.a4 * (m.b1 * m.c3 - m.b3 * m.c1)),

      c1:  inv_det * (m.b1 * (m.c2 * m.d4 - m.c4 * m.d2) + m.b2 * (m.c4 * m.d1 - m.c1 * m.d4) + m.b4 * (m.c1 * m.d2 - m.c2 * m.d1)),
      c2: -inv_det * (m.a1 * (m.c2 * m.d4 - m.c4 * m.d2) + m.a2 * (m.c4 * m.d1 - m.c1 * m.d4) + m.a4 * (m.c1 * m.d2 - m.c2 * m.d1)),
      c3:  inv_det * (m.a1 * (m.b2 * m.d4 - m.b4 * m.d2) + m.a2 * (m.b4 * m.d1 - m.b1 * m.d4) + m.a4 * (m.b1 * m.d2 - m.b2 * m.d1)),
      c4: -inv_det * (m.a1 * (m.b2 * m.c4 - m.b4 * m.c2) + m.a2 * (m.b4 * m.c1 - m.b1 * m.c4) + m.a4 * (m.b1 * m.c2 - m.b2 * m.c1)),
      
      d1: -inv_det * (m.b1 * (m.c2 * m.d3 - m.c3 * m.d2) + m.b2 * (m.c3 * m.d1 - m.c1 * m.d3) + m.b3 * (m.c1 * m.d2 - m.c2 * m.d1)),
      d2:  inv_det * (m.a1 * (m.c2 * m.d3 - m.c3 * m.d2) + m.a2 * (m.c3 * m.d1 - m.c1 * m.d3) + m.a3 * (m.c1 * m.d2 - m.c2 * m.d1)),
      d3: -inv_det * (m.a1 * (m.b2 * m.d3 - m.b3 * m.d2) + m.a2 * (m.b3 * m.d1 - m.b1 * m.d3) + m.a3 * (m.b1 * m.d2 - m.b2 * m.d1)),
      d4:  inv_det * (m.a1 * (m.b2 * m.c3 - m.b3 * m.c2) + m.a2 * (m.b3 * m.c1 - m.b1 * m.c3) + m.a3 * (m.b1 * m.c2 - m.b2 * m.c1)),
    }
    .into()
  }

  fn max_scale(&self) -> T {
    let x = self.a1 * self.a1 + self.a2 * self.a2 + self.a3 * self.a3;
    let y = self.b1 * self.b1 + self.b2 * self.b2 + self.b3 * self.b3;
    let z = self.c1 * self.c1 + self.c2 * self.c2 + self.c3 * self.c3;
    x.max(y).max(z).sqrt()
  }
}

impl<T> Mat4<T> {
  pub fn to_mat3(self) -> Mat3<T> {
    #[rustfmt::skip]
    Mat3 {
      a1: self.a1, a2: self.a2, a3: self.a3,
      b1: self.b1, b2: self.b2, b3: self.b3,
      c1: self.c1, c2: self.c2, c3: self.c3,
    }
  }
}

impl<T: Scalar> Mat4<T> {
  pub fn to_normal_matrix(self) -> Mat3<T> {
    self.to_mat3().inverse().unwrap().transpose()
  }
}

unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Mat4<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Mat4<T> {}

impl<T> Mul<Vec3<T>> for Mat4<T>
where
  T: Copy + Add<Output = T> + Mul<Output = T> + Div<Output = T> + num_traits::One,
{
  type Output = Vec3<T>;

  fn mul(self, v: Vec3<T>) -> Vec3<T> {
    let v = Vec4::new(v.x, v.y, v.z, T::one());
    let v = self * v;
    Vec3::new(v.x, v.y, v.z) / v.w
  }
}

impl<T: Scalar> SpaceEntity<T, 3> for Vec3<T> {
  type Matrix = Mat4<T>;
  #[inline(always)]
  fn apply_matrix(&mut self, m: Self::Matrix) -> &mut Self {
    *self = m * *self;
    self
  }
}

impl<T> Mul<Vec4<T>> for Mat4<T>
where
  T: Copy + Add<Output = T> + Mul<Output = T>,
{
  type Output = Vec4<T>;

  fn mul(self, v: Vec4<T>) -> Vec4<T> {
    Vec4 {
      x: (v.x * self.a1 + v.y * self.b1 + v.z * self.c1 + v.w * self.d1),
      y: (v.x * self.a2 + v.y * self.b2 + v.z * self.c2 + v.w * self.d2),
      z: (v.x * self.a3 + v.y * self.b3 + v.z * self.c3 + v.w * self.d3),
      w: (v.x * self.a4 + v.y * self.b4 + v.z * self.c4 + v.w * self.d4),
    }
  }
}

impl<T> Mul for Mat4<T>
where
  T: Copy + Mul<Output = T> + Add<Output = T>,
{
  type Output = Mat4<T>;

  fn mul(self, m: Self) -> Self {
    let a = self;

    Self {
      a1: a.a1 * m.a1 + a.b1 * m.a2 + a.c1 * m.a3 + a.d1 * m.a4,
      a2: a.a2 * m.a1 + a.b2 * m.a2 + a.c2 * m.a3 + a.d2 * m.a4,
      a3: a.a3 * m.a1 + a.b3 * m.a2 + a.c3 * m.a3 + a.d3 * m.a4,
      a4: a.a4 * m.a1 + a.b4 * m.a2 + a.c4 * m.a3 + a.d4 * m.a4,

      b1: a.a1 * m.b1 + a.b1 * m.b2 + a.c1 * m.b3 + a.d1 * m.b4,
      b2: a.a2 * m.b1 + a.b2 * m.b2 + a.c2 * m.b3 + a.d2 * m.b4,
      b3: a.a3 * m.b1 + a.b3 * m.b2 + a.c3 * m.b3 + a.d3 * m.b4,
      b4: a.a4 * m.b1 + a.b4 * m.b2 + a.c4 * m.b3 + a.d4 * m.b4,

      c1: a.a1 * m.c1 + a.b1 * m.c2 + a.c1 * m.c3 + a.d1 * m.c4,
      c2: a.a2 * m.c1 + a.b2 * m.c2 + a.c2 * m.c3 + a.d2 * m.c4,
      c3: a.a3 * m.c1 + a.b3 * m.c2 + a.c3 * m.c3 + a.d3 * m.c4,
      c4: a.a4 * m.c1 + a.b4 * m.c2 + a.c4 * m.c3 + a.d4 * m.c4,

      d1: a.a1 * m.d1 + a.b1 * m.d2 + a.c1 * m.d3 + a.d1 * m.d4,
      d2: a.a2 * m.d1 + a.b2 * m.d2 + a.c2 * m.d3 + a.d2 * m.d4,
      d3: a.a3 * m.d1 + a.b3 * m.d2 + a.c3 * m.d3 + a.d3 * m.d4,
      d4: a.a4 * m.d1 + a.b4 * m.d2 + a.c4 * m.d3 + a.d4 * m.d4,
    }
  }
}

#[test]
fn mul() {
  let cgmath_mat1 = cgmath::Matrix4::<f32>::from_translation(cgmath::vec3(1., 2., 3.));
  let cgmath_mat2 = cgmath::Matrix4::<f32>::from_nonuniform_scale(3., -2., 3.);
  let cgmath_point = cgmath::vec4(1., 2., 3., 1.);
  let cgmath_r = cgmath_mat1 * cgmath_mat2 * cgmath_point;
  let cgmath_r: [f32; 4] = *cgmath_r.as_ref();

  let math_mat1 = Mat4::<f32>::translate((1., 2., 3.));
  let math_mat2 = Mat4::<f32>::scale((3., -2., 3.));
  let math_point = Vec4::new(1., 2., 3., 1.);
  let math_r = math_mat1 * math_mat2 * math_point;
  let math_r: [f32; 4] = math_r.into();

  assert_eq!(cgmath_r, math_r)
}

#[rustfmt::skip]
impl<T: Sized> Mat4<T> {
  pub const fn new(
    m11: T, m12: T, m13: T, m14: T,
    m21: T, m22: T, m23: T, m24: T,
    m31: T, m32: T, m33: T, m34: T,
    m41: T, m42: T, m43: T, m44: T,
  ) -> Self {
    Self {
      a1: m11, a2: m12, a3: m13, a4: m14,
      b1: m21, b2: m22, b3: m23, b4: m24,
      c1: m31, c2: m32, c3: m33, c4: m34,
      d1: m41, d2: m42, d3: m43, d4: m44,
    }
  }
}

impl<T> Mat4<T>
where
  T: Scalar,
{
  pub fn from_orth_basis_and_position(forward: Vec3<T>, up: Vec3<T>, position: Vec3<T>) -> Self {
    let right = forward.cross(up);
    #[rustfmt::skip]
    Mat4::new(
      right.x,    right.y,    right.z,     T::zero(), 
      up.x,       up.y,       up.z,        T::zero(), 
      forward.x,  forward.y,  forward.z,   T::zero(), 
      position.x, position.y,  position.z, T::one(),
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

impl<T> Mat4<T>
where
  T: Scalar,
{
  pub fn rotate_x(theta: T) -> Self {
    let (sin, cos) = theta.sin_cos();

    #[rustfmt::skip]
    Mat4::new(
      T::one(),   T::zero(), T::zero(), T::zero(), 
      T::zero(),  cos,       sin,       T::zero(), 
      T::zero(), -sin,       cos,       T::zero(), 
      T::zero(),  T::zero(), T::zero(), T::one(),
    )
  }

  pub fn rotate_y(theta: T) -> Self {
    let (sin, cos) = theta.sin_cos();

    #[rustfmt::skip]
    Mat4::new(
      cos,       T::zero(), -sin,       T::zero(), 
      T::zero(), T::one(),   T::zero(), T::zero(), 
      sin,       T::zero(),  cos,       T::zero(), 
      T::zero(), T::zero(),  T::zero(), T::one(),
    )
  }

  pub fn rotate_z(theta: T) -> Self {
    let (sin, cos) = theta.sin_cos();

    #[rustfmt::skip]
    Mat4::new(
      cos,       sin,       T::zero(), T::zero(), 
     -sin,       cos,       T::zero(), T::zero(), 
      T::zero(), T::zero(), T::one(),  T::zero(), 
      T::zero(), T::zero(), T::zero(), T::one(),
    )
  }

  pub fn rotate(axis: Vec3<T>, theta: T) -> Self {
    let (sin, cos) = theta.sin_cos();

    let x = axis.x;
    let y = axis.y;
    let z = axis.z;

    let t = T::one() - cos;
    let tx = t * x;
    let ty = t * y;
    let tz = t * z;

    #[rustfmt::skip]
    Mat4::new(
      tx * x + cos,     tx * y + sin * z, tx * z - sin * y, T::zero(),
      tx * y - sin * z, ty * y + cos,     ty * z + sin * x, T::zero(),
      tx * z + sin * y, ty * z - sin * x, tz * z + cos,     T::zero(),
      T::zero(),        T::zero(),        T::zero(),        T::one(),
    )
  }

  pub fn scale(s: impl Into<Vec3<T>>) -> Self {
    let Vec3 { x, y, z } = s.into();
    let zero = T::zero();
    let one = T::one();

    #[rustfmt::skip]
    Mat4::new(
      x,    zero, zero, zero, 
      zero, y,    zero, zero, 
      zero, zero, z,    zero, 
      zero, zero, zero, one,
    )
  }

  pub fn translate(s: impl Into<Vec3<T>>) -> Self {
    let Vec3 { x, y, z } = s.into();
    let zero = T::zero();
    let one = T::one();

    #[rustfmt::skip]
    Mat4::new(
      one,  zero, zero, zero, 
      zero, one,  zero, zero, 
      zero, zero, one,  zero, 
      x,    y,    z,    one,
    )
  }

  pub fn transform_inverse(&self) -> Option<Self> {
    let m = self;
    let det = (m.a1 * m.b2 - m.a2 * m.b1) * (m.c3) - (m.a1 * m.b3 - m.a3 * m.b1) * (m.c2)
      + (m.a2 * m.b3 - m.a3 * m.b2) * (m.c1);
    if det == T::zero() {
      return None;
    }

    let inv_det = T::one() / det;

    Some(Self {
      a1: inv_det * (m.b2 * m.c3 + m.b3 * -m.c2),
      a2: inv_det * (m.c2 * m.a3 + m.c3 * -m.a2),
      a3: inv_det * (m.a2 * m.b3 - m.a3 * m.b2),
      a4: T::zero(),
      b1: inv_det * (m.b3 * m.c1 + m.b1 * -m.c3),
      b2: inv_det * (m.c3 * m.a1 + m.c1 * -m.a3),
      b3: inv_det * (m.a3 * m.b1 - m.a1 * m.b3),
      b4: T::zero(),
      c1: inv_det * (m.b1 * m.c2 + m.b2 * -m.c1),
      c2: inv_det * (m.c1 * m.a2 + m.c2 * -m.a1),
      c3: inv_det * (m.a1 * m.b2 - m.a2 * m.b1),
      c4: T::zero(),
      d1: inv_det
        * (m.b1 * (m.c3 * m.d2 - m.c2 * m.d3)
          + m.b2 * (m.c1 * m.d3 - m.c3 * m.d1)
          + m.b3 * (m.c2 * m.d1 - m.c1 * m.d2)),
      d2: inv_det
        * (m.c1 * (m.a3 * m.d2 - m.a2 * m.d3)
          + m.c2 * (m.a1 * m.d3 - m.a3 * m.d1)
          + m.c3 * (m.a2 * m.d1 - m.a1 * m.d2)),
      d3: inv_det
        * (m.d1 * (m.a3 * m.b2 - m.a2 * m.b3)
          + m.d2 * (m.a1 * m.b3 - m.a3 * m.b1)
          + m.d3 * (m.a2 * m.b1 - m.a1 * m.b2)),
      d4: inv_det
        * (m.a1 * (m.b2 * m.c3 - m.b3 * m.c2)
          + m.a2 * (m.b3 * m.c1 - m.b1 * m.c3)
          + m.a3 * (m.b1 * m.c2 - m.b2 * m.c1)),
    })
  }

  pub fn lookat(eye: Vec3<T>, center: Vec3<T>, up: Vec3<T>) -> Self {
    let mut z = eye - center;
    z = z.normalize();

    let mut x = up.cross(z);
    x = x.normalize();

    let mut y = z.cross(x);
    y = y.normalize();

    #[rustfmt::skip]
    Mat4::new(
        x.x,   x.y,   x.z, T::zero(),
        y.x,   y.y,   y.z, T::zero(),
        z.x,   z.y,   z.z, T::zero(),
      eye.x, eye.y, eye.z, T::one(),
    )
  }

  #[must_use]
  pub fn extract_rotation_mat(&self) -> Self {
    let scale_inv = self.get_scale().map(|v| T::one() / v);
    let mut mat = self.to_mat3();
    mat.a1 *= scale_inv.x;
    mat.a2 *= scale_inv.x;
    mat.a3 *= scale_inv.x;

    mat.b1 *= scale_inv.y;
    mat.b2 *= scale_inv.y;
    mat.b3 *= scale_inv.y;

    mat.c1 *= scale_inv.z;
    mat.c2 *= scale_inv.z;
    mat.c3 *= scale_inv.z;
    mat.into()
  }

  pub fn extract_scale(&self) -> Vec3<T> {
    let sx = Vec3::new(self.a1, self.a2, self.a3).length();
    let sy = Vec3::new(self.b1, self.b2, self.b3).length();
    let sz = Vec3::new(self.c1, self.c2, self.c3).length();
    Vec3::new(sx, sy, sz)
  }
}

impl<T> num_traits::Zero for Mat4<T>
where
  T: num_traits::Zero + Copy + PartialEq,
{
  #[inline(always)]
  fn zero() -> Self {
    #[rustfmt::skip]
    Self {
      a1: T::zero(), a2: T::zero(), a3: T::zero(), a4: T::zero(),
      b1: T::zero(), b2: T::zero(), b3: T::zero(), b4: T::zero(),
      c1: T::zero(), c2: T::zero(), c3: T::zero(), c4: T::zero(),
      d1: T::zero(), d2: T::zero(), d3: T::zero(), d4: T::zero(),
    }
  }
  #[inline(always)]
  fn is_zero(&self) -> bool {
    self.eq(&Self::zero())
  }
}

impl<T> num_traits::One for Mat4<T>
where
  T: num_traits::One + num_traits::Zero + Copy,
{
  #[inline(always)]
  fn one() -> Self {
    #[rustfmt::skip]
    Self {
      a1: T::one(),  a2: T::zero(), a3: T::zero(), a4: T::zero(),
      b1: T::zero(), b2: T::one(),  b3: T::zero(), b4: T::zero(),
      c1: T::zero(), c2: T::zero(), c3: T::one(),  c4: T::zero(),
      d1: T::zero(), d2: T::zero(), d3: T::zero(), d4: T::one(),
    }
  }
}

impl<T: Scalar> From<Mat3<T>> for Mat4<T> {
  fn from(m: Mat3<T>) -> Self {
    #[rustfmt::skip]
    Self {
      a1: m.a1,      a2: m.a2,      a3: m.a3,      a4: T::zero(),
      b1: m.b1,      b2: m.b2,      b3: m.b3,      b4: T::zero(),
      c1: m.c1,      c2: m.c2,      c3: m.c3,      c4: T::zero(),
      d1: T::zero(), d2: T::zero(), d3: T::zero(), d4: T::one(),
    }
  }
}

impl<T: Scalar> From<Quat<T>> for Mat4<T> {
  fn from(q: Quat<T>) -> Self {
    let (xs, ys, zs) = (q.x * T::two(), q.y * T::two(), q.z * T::two());

    let (xx, xy, xz) = (q.x * xs, q.x * ys, q.x * zs);
    let (yy, yz, zz) = (q.y * ys, q.y * zs, q.z * zs);
    let (wx, wy, wz) = (q.w * xs, q.w * ys, q.w * zs);

    #[rustfmt::skip]
    Self {
      a1: T::one() - (yy + zz), a2: xy + wz,              a3: xz - wy,              a4: T::zero(),
      b1: xy - wz,              b2: T::one() - (xx + zz), b3: yz + wx,              b4: T::zero(),
      c1: xz + wy,              c2: yz - wx,              c3: T::one() - (xx + yy), c4: T::zero(),
      d1: T::zero(),            d2: T::zero(),            d3: T::zero(),            d4: T::one(),
    }
  }
}

impl<T: Scalar> Mat4<T> {
  /// Mat should be TRS composed, return (translate, quaternion, scale)
  pub fn decompose(&self) -> (Vec3<T>, Quat<T>, Vec3<T>) {
    let mut scale = self.get_scale();

		// if determine is negative, we need to invert one scale
		if self.det() < T::zero() {
      scale.x *= -T::one()
    }

    let position = self.position();

    let quaternion = self.extract_rotation_mat().to_mat3().into();

    (position, quaternion, scale)
  }
}

impl<T> AsRef<Mat4<T>> for Mat4<T> {
  fn as_ref(&self) -> &Mat4<T> {
    self
  }
}

impl<T> AsMut<Mat4<T>> for Mat4<T> {
  fn as_mut(&mut self) -> &mut Mat4<T> {
    self
  }
}
