use crate::*;
use std::fmt::Debug;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Quat<T> {
  pub x: T,
  pub y: T,
  pub z: T,
  pub w: T,
}

impl<T> Quat<T> {
  pub fn set(&mut self, x: T, y: T, z: T, w: T) -> &Self {
    self.x = x;
    self.y = y;
    self.z = z;
    self.w = w;
    self
  }
}

impl<T> Neg for Quat<T>
where
  T: Neg<Output = T>,
{
  type Output = Self;

  fn neg(self) -> Self {
    Self {
      x: -self.x,
      y: -self.y,
      z: -self.z,
      w: -self.w,
    }
  }
}

impl<T> Add for Quat<T>
where
  T: Add<Output = T>,
{
  type Output = Self;

  fn add(self, other: Self) -> Self {
    Self {
      x: self.x + other.x,
      y: self.y + other.y,
      z: self.z + other.z,
      w: self.w + other.w,
    }
  }
}

impl<T> Sub for Quat<T>
where
  T: Sub<Output = T>,
{
  type Output = Self;

  fn sub(self, other: Self) -> Self {
    Self {
      x: self.x - other.x,
      y: self.y - other.y,
      z: self.z - other.z,
      w: self.w - other.w,
    }
  }
}

impl<T> Mul<T> for Quat<T>
where
  T: Mul<Output = T> + Copy,
{
  type Output = Self;

  fn mul(self, s: T) -> Self {
    Self {
      x: self.x * s,
      y: self.y * s,
      z: self.z * s,
      w: self.w * s,
    }
  }
}

impl<T> Mul for Quat<T>
where
  T: Scalar,
{
  type Output = Self;

  fn mul(self, other: Self) -> Self {
    let (qax, qay, qaz, qaw) = self.into();
    let (qbx, qby, qbz, qbw) = other.into();
    Self {
      x: qax * qbw + qaw * qbx + qay * qbz - qaz * qby,
      y: qay * qbw + qaw * qby + qaz * qbx - qax * qbz,
      z: qaz * qbw + qaw * qbz + qax * qby - qay * qbx,
      w: qaw * qbw - qax * qbx - qay * qby - qaz * qbz,
    }
  }
}

impl<T> Div<T> for Quat<T>
where
  T: Div<Output = T> + Copy,
{
  type Output = Self;

  fn div(self, s: T) -> Self {
    Self {
      x: self.x / s,
      y: self.y / s,
      z: self.z / s,
      w: self.w / s,
    }
  }
}

impl<T> Quat<T>
where
  T: Copy,
{
  /// Creates a new Quat from multiple components
  pub fn new(x: T, y: T, z: T, w: T) -> Self {
    Self { x, y, z, w }
  }
}

impl<T> Quat<T>
where
  T: Scalar,
{
  pub fn rotation_x(theta: T) -> Self {
    let theta_half = theta * T::half();

    Self {
      w: theta_half.cos(),
      x: theta_half.sin(),
      y: T::zero(),
      z: T::zero(),
    }
  }

  pub fn rotation_y(theta: T) -> Self {
    let theta_half = theta * T::half();

    Self {
      w: theta_half.cos(),
      x: T::zero(),
      y: theta_half.sin(),
      z: T::zero(),
    }
  }

  pub fn rotation_z(theta: T) -> Self {
    let theta_half = theta * T::half();

    Self {
      w: theta_half.cos(),
      x: T::zero(),
      y: T::zero(),
      z: theta_half.sin(),
    }
  }

  pub fn rotation(axis: &Vec3<T>, theta: T) -> Self {
    let (s, c) = (theta * T::half()).sin_cos();

    Self {
      w: c,
      x: axis.x * s,
      y: axis.y * s,
      z: axis.z * s,
    }
  }

  pub fn direction(a: &Vec3<T>, b: &Vec3<T>) -> Self {
    let axis = a.cross(*b);
    let cos_angle = a.dot(*b);

    let t0 = T::one() + cos_angle;
    let t1 = (t0 + t0).sqrt().recip();
    let t2 = (t0 + t0) * t1 * T::half();

    Self {
      x: axis.x * t1,
      y: axis.y * t1,
      z: axis.z * t1,
      w: t2,
    }
  }

  pub fn dot(&self, b: Self) -> T {
    self.x * b.x + self.y * b.y + self.z * b.z + self.w * b.w
  }

  pub fn length2(&self) -> T {
    self.dot(*self)
  }

  pub fn length(&self) -> T {
    self.length2().sqrt()
  }

  pub fn distance(&self, b: Self) -> T {
    (*self - b).length()
  }

  #[must_use]
  pub fn normalize(&self) -> Self {
    let mag_sq = self.length2();
    if mag_sq > T::zero() {
      let inv_sqrt = T::one() / mag_sq.sqrt();
      *self * inv_sqrt
    } else {
      (T::zero(), T::zero(), T::zero(), T::one()).into()
    }
  }

  pub fn axis(&self) -> Vec3<T> {
    let sin_theta_over2_sq = T::one() - self.w * self.w;
    if sin_theta_over2_sq <= T::zero() {
      return Vec3::new(T::one(), T::zero(), T::zero());
    }

    let v = Vec3::new(self.x, self.y, self.z);
    let inv_sqrt = T::one() / sin_theta_over2_sq.sqrt();

    v * Vec3::new(inv_sqrt, inv_sqrt, inv_sqrt)
  }

  pub fn angle(&self) -> T {
    self.w.acos() * T::two()
  }

  #[must_use]
  pub fn conjugate(&self) -> Self {
    Self {
      x: -self.x,
      y: -self.y,
      z: -self.z,
      w: self.w,
    }
  }

  #[must_use]
  pub fn inverse(&self) -> Self {
    self.conjugate()
  }
}

impl<T: Scalar> From<Mat3<T>> for Quat<T> {
  /// assume mat3 is unscaled pure rotation
  fn from(mat: Mat3<T>) -> Self {
    #[rustfmt::skip]
    let Mat3 { a1, a2, a3, b1, b2, b3, c1, c2, c3 } = mat;

    let trace = a1 + b2 + c3;

    if trace > T::zero() {
      let s = T::half() / (trace + T::one()).sqrt();

      let w = T::half() * T::half() / s;
      let x = (b3 - c2) * s;
      let y = (c1 - a3) * s;
      let z = (a2 - b1) * s;
      (x, y, z, w)
    } else if a1 > b2 && a1 > c3 {
      let s = T::two() * (T::one() + a1 - b2 - c3).sqrt();

      let w = (b3 - c2) / s;
      let x = T::half() * T::half() * s;
      let y = (b1 + a2) / s;
      let z = (c1 + a3) / s;
      (x, y, z, w)
    } else if b2 > c3 {
      let s = T::two() * (T::one() + b2 - a1 - c3).sqrt();

      let w = (c1 - a3) / s;
      let x = (b1 + a2) / s;
      let y = T::half() * T::half() * s;
      let z = (c2 + b3) / s;
      (x, y, z, w)
    } else {
      let s = T::two() * (T::one() + c3 - a1 - b2).sqrt();

      let w = (a2 - b1) / s;
      let x = (c1 + a3) / s;
      let y = (c2 + b3) / s;
      let z = T::half() * T::half() * s;
      (x, y, z, w)
    }
    .into()
  }
}

impl<T> Slerp<T> for Quat<T>
where
  T: Scalar,
{
  fn slerp(self, _other: Self, _factor: T) -> Self {
    todo!()
  }
}

impl<T> num_traits::Zero for Quat<T>
where
  T: num_traits::Zero + PartialEq,
{
  #[inline(always)]
  fn zero() -> Self {
    Self {
      x: T::zero(),
      y: T::zero(),
      z: T::zero(),
      w: T::zero(),
    }
  }
  #[inline(always)]
  fn is_zero(&self) -> bool {
    self.eq(&Self::zero())
  }
}

impl<T> num_traits::One for Quat<T>
where
  T: Scalar,
{
  #[inline(always)]
  fn one() -> Self {
    Self {
      x: T::one(),
      y: T::one(),
      z: T::one(),
      w: T::one(),
    }
  }
}

impl<T> From<Vec4<T>> for Quat<T>
where
  T: Copy,
{
  fn from(v: Vec4<T>) -> Self {
    Self {
      x: v.x,
      y: v.y,
      z: v.z,
      w: v.w,
    }
  }
}

impl<T> From<[T; 4]> for Quat<T>
where
  T: Copy,
{
  fn from(v: [T; 4]) -> Self {
    Self {
      x: v[0],
      y: v[1],
      z: v[2],
      w: v[3],
    }
  }
}

impl<T> From<(T, T, T, T)> for Quat<T>
where
  T: Copy,
{
  fn from(v: (T, T, T, T)) -> Self {
    Self {
      x: v.0,
      y: v.1,
      z: v.2,
      w: v.3,
    }
  }
}

impl<T> Into<(T, T, T, T)> for Quat<T> {
  fn into(self) -> (T, T, T, T) {
    (self.x, self.y, self.z, self.w)
  }
}

impl<T> AsRef<Quat<T>> for Quat<T> {
  fn as_ref(&self) -> &Quat<T> {
    self
  }
}

impl<T> AsMut<Quat<T>> for Quat<T> {
  fn as_mut(&mut self) -> &mut Quat<T> {
    self
  }
}
