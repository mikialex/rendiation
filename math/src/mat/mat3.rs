use crate::*;
use std::ops::{Add, Mul};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Mat3<T> {
  pub a1: T,
  pub a2: T,
  pub a3: T,
  pub b1: T,
  pub b2: T,
  pub b3: T,
  pub c1: T,
  pub c2: T,
  pub c3: T,
}

impl<T: Scalar> SquareMatrixDimension<2> for Mat3<T> {}
impl<T: Scalar> SquareMatrix<T> for Mat3<T> {}

unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Mat3<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Mat3<T> {}

impl<T> Mul<Mat3<T>> for Vec2<T>
where
  T: Copy + Add<Output = T> + Mul<Output = T> + One,
{
  type Output = Self;

  fn mul(self, m: Mat3<T>) -> Self {
    Self {
      x: self.x * m.a1 + self.y * m.b1 + m.c1,
      y: self.x * m.a2 + self.y * m.b2 + m.c2,
    }
  }
}
impl<T: Scalar> SpaceEntity<T, 2> for Vec2<T> {
  #[inline(always)]
  fn apply_matrix(&mut self, m: &SquareMatrixType<T, 2>) -> &mut Self {
    *self = *self * *m;
    self
  }
}

impl<T> Mul for Mat3<T>
where
  T: Copy + Mul<Output = T> + Add<Output = T>,
{
  type Output = Self;

  fn mul(self, m: Self) -> Self {
    let a = self;

    Self {
      a1: a.a1 * m.a1 + a.b1 * m.a2 + a.c1 * m.a3,
      a2: a.a2 * m.a1 + a.b2 * m.a2 + a.c2 * m.a3,
      a3: a.a3 * m.a1 + a.b3 * m.a2 + a.c3 * m.a3,

      b1: a.a1 * m.b1 + a.b1 * m.b2 + a.c1 * m.b3,
      b2: a.a2 * m.b1 + a.b2 * m.b2 + a.c2 * m.b3,
      b3: a.a3 * m.b1 + a.b3 * m.b2 + a.c3 * m.b3,

      c1: a.a1 * m.c1 + a.b1 * m.c2 + a.c1 * m.c3,
      c2: a.a2 * m.c1 + a.b2 * m.c2 + a.c2 * m.c3,
      c3: a.a3 * m.c1 + a.b3 * m.c2 + a.c3 * m.c3,
    }
  }
}

impl<T> Mat3<T>
where
  T: Copy,
{
  #[clippy::skip]
  pub fn new(m11: T, m12: T, m13: T, m21: T, m22: T, m23: T, m31: T, m32: T, m33: T) -> Self {
    Self {
      a1: m11,
      a2: m12,
      a3: m13,
      b1: m21,
      b2: m22,
      b3: m23,
      c1: m31,
      c2: m32,
      c3: m33,
    }
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
}

impl<T> Mat3<T>
where
  T: Arithmetic + Math,
{
  pub fn det(&self) -> T {
    let t11 = self.c3 * self.b2 - self.b3 * self.c2;
    let t12 = self.b3 * self.c1 - self.c3 * self.b1;
    let t13 = self.c2 * self.b1 - self.b2 * self.c1;
    self.a1 * t11 + self.a2 * t12 + self.a3 * t13
  }

  pub fn inverse(&self) -> Option<Self> {
    let det = self.det();
    if det == T::zero() {
      return None;
    }

    let invdet = T::one() / det;

    Some(Self {
      a1: (self.c3 * self.b2 - self.b3 * self.c2) * invdet,
      a2: (self.a3 * self.c2 - self.c3 * self.a2) * invdet,
      a3: (self.b3 * self.a2 - self.a3 * self.b2) * invdet,
      b1: (self.b3 * self.c1 - self.c3 * self.b1) * invdet,
      b2: (self.c3 * self.a1 - self.a3 * self.c1) * invdet,
      b3: (self.a3 * self.b1 - self.b3 * self.a1) * invdet,
      c1: (self.c2 * self.b1 - self.b2 * self.c1) * invdet,
      c2: (self.a2 * self.c1 - self.c2 * self.a1) * invdet,
      c3: (self.b2 * self.a1 - self.a2 * self.b1) * invdet,
    })
  }

  pub fn transpose(&self) -> Mat3<T> {
    Mat3::new(
      self.a1, self.b1, self.c1, self.a2, self.b2, self.c2, self.a3, self.b3, self.c3,
    )
  }

  pub fn rotate_x(theta: T) -> Self {
    let (s, c) = theta.sincos();

    let a1 = T::one();
    let a2 = T::zero();
    let a3 = T::zero();

    let b1 = T::zero();
    let b2 = c;
    let b3 = s;

    let c1 = T::zero();
    let c2 = -s;
    let c3 = c;

    Mat3::new(a1, a2, a3, b1, b2, b3, c1, c2, c3)
  }

  pub fn rotate_y(theta: T) -> Self {
    let (s, c) = theta.sincos();

    let a1 = c;
    let a2 = T::zero();
    let a3 = -s;

    let b1 = T::zero();
    let b2 = T::one();
    let b3 = T::zero();

    let c1 = s;
    let c2 = T::zero();
    let c3 = c;

    Mat3::new(a1, a2, a3, b1, b2, b3, c1, c2, c3)
  }

  pub fn rotate_z(theta: T) -> Self {
    let (s, c) = theta.sincos();

    let a1 = c;
    let a2 = s;
    let a3 = T::zero();

    let b1 = -s;
    let b2 = c;
    let b3 = T::zero();

    let c1 = T::zero();
    let c2 = T::zero();
    let c3 = T::one();

    Mat3::new(a1, a2, a3, b1, b2, b3, c1, c2, c3)
  }

  pub fn rotate(axis: Vec3<T>, theta: T) -> Self {
    let (s, c) = theta.sincos();

    let x = axis.x;
    let y = axis.y;
    let z = axis.z;

    let t = T::one() - c;
    let tx = t * x;
    let ty = t * y;
    let tz = t * z;

    let a1 = tx * x + c;
    let a2 = tx * y + s * z;
    let a3 = tx * z - s * y;

    let b1 = tx * y - s * z;
    let b2 = ty * y + c;
    let b3 = ty * z + s * x;

    let c1 = tx * z + s * y;
    let c2 = ty * z - s * x;
    let c3 = tz * z + c;

    Mat3::new(a1, a2, a3, b1, b2, b3, c1, c2, c3)
  }

  pub fn scale(x: T, y: T, z: T) -> Self {
    let (a1, a2, a3) = (x, T::zero(), T::zero());
    let (b1, b2, b3) = (T::zero(), y, T::zero());
    let (c1, c2, c3) = (T::zero(), T::zero(), z);

    Mat3::new(a1, a2, a3, b1, b2, b3, c1, c2, c3)
  }

  pub fn translate(x: T, y: T) -> Self {
    let (a1, a2, a3) = (T::one(), T::zero(), T::zero());
    let (b1, b2, b3) = (T::zero(), T::one(), T::one());
    let (c1, c2, c3) = (x, y, T::one());

    Mat3::new(a1, a2, a3, b1, b2, b3, c1, c2, c3)
  }
}

impl<T> Zero for Mat3<T>
where
  T: Zero,
{
  #[inline(always)]
  fn zero() -> Self {
    Self {
      a1: T::zero(),
      a2: T::zero(),
      a3: T::zero(),
      b1: T::zero(),
      b2: T::zero(),
      b3: T::zero(),
      c1: T::zero(),
      c2: T::zero(),
      c3: T::zero(),
    }
  }
}

impl<T> One for Mat3<T>
where
  T: One + Zero,
{
  #[inline(always)]
  fn one() -> Self {
    Self {
      a1: T::one(),
      a2: T::zero(),
      a3: T::zero(),
      b1: T::zero(),
      b2: T::one(),
      b3: T::zero(),
      c1: T::zero(),
      c2: T::zero(),
      c3: T::one(),
    }
  }
}

impl<T: Arithmetic> From<Quat<T>> for Mat3<T> {
  fn from(q: Quat<T>) -> Self {
    let (xs, ys, zs) = (q.x * T::two(), q.y * T::two(), q.z * T::two());

    let (xx, xy, xz) = (q.x * xs, q.x * ys, q.x * zs);
    let (yy, yz, zz) = (q.y * ys, q.y * zs, q.z * zs);
    let (wx, wy, wz) = (q.w * xs, q.w * ys, q.w * zs);

    Self {
      a1: T::one() - (yy + zz),
      a2: xy + wz,
      a3: xz - wy,

      b1: xy - wz,
      b2: T::one() - (xx + zz),
      b3: yz + wx,

      c1: xz + wy,
      c2: yz - wx,
      c3: T::one() - (xx + yy),
    }
  }
}

impl<T> From<(T, T, T, T, T, T, T, T, T)> for Mat3<T>
where
  T: Copy,
{
  fn from(v: (T, T, T, T, T, T, T, T, T)) -> Self {
    Self {
      a1: v.0,
      a2: v.1,
      a3: v.2,
      b1: v.3,
      b2: v.4,
      b3: v.5,
      c1: v.6,
      c2: v.7,
      c3: v.8,
    }
  }
}

impl<T> AsRef<Mat3<T>> for Mat3<T> {
  fn as_ref(&self) -> &Mat3<T> {
    self
  }
}

impl<T> AsMut<Mat3<T>> for Mat3<T> {
  fn as_mut(&mut self) -> &mut Mat3<T> {
    self
  }
}
