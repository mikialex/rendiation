use crate::*;
use std::fmt;
use std::fmt::Debug;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

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
  T: Mul<Output = T>,
{
  type Output = Self;

  fn mul(self, other: Self) -> Self {
    Self {
      x: self.x * other.x,
      y: self.y * other.y,
      z: self.z * other.z,
      w: self.w * other.w,
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

impl<T> Div for Quat<T>
where
  T: Div<Output = T>,
{
  type Output = Self;

  fn div(self, other: Self) -> Self {
    Self {
      x: self.x / other.x,
      y: self.y / other.y,
      z: self.z / other.z,
      w: self.w / other.w,
    }
  }
}

impl<T> AddAssign for Quat<T>
where
  T: AddAssign<T>,
{
  fn add_assign(&mut self, other: Self) {
    self.x += other.x;
    self.y += other.y;
    self.z += other.z;
    self.w += other.w;
  }
}

impl<T> SubAssign for Quat<T>
where
  T: SubAssign<T>,
{
  fn sub_assign(&mut self, other: Self) {
    self.x -= other.x;
    self.y -= other.y;
    self.z -= other.z;
    self.w -= other.w;
  }
}

impl<T> MulAssign<T> for Quat<T>
where
  T: MulAssign<T> + Copy,
{
  fn mul_assign(&mut self, s: T) {
    self.x *= s;
    self.y *= s;
    self.z *= s;
    self.w *= s;
  }
}

impl<T> MulAssign for Quat<T>
where
  T: MulAssign<T>,
{
  fn mul_assign(&mut self, other: Self) {
    self.x *= other.x;
    self.y *= other.y;
    self.z *= other.z;
    self.w *= other.w;
  }
}

impl<'a, T> MulAssign<&'a T> for Quat<T>
where
  T: MulAssign<T> + Copy,
{
  fn mul_assign(&mut self, other: &'a T) {
    self.x *= *other;
    self.y *= *other;
    self.z *= *other;
    self.w *= *other;
  }
}

impl<T> DivAssign<T> for Quat<T>
where
  T: DivAssign<T> + Copy,
{
  fn div_assign(&mut self, s: T) {
    self.x /= s;
    self.y /= s;
    self.z /= s;
    self.w /= s;
  }
}

impl<T> DivAssign for Quat<T>
where
  T: DivAssign<T>,
{
  fn div_assign(&mut self, other: Self) {
    self.x /= other.x;
    self.y /= other.y;
    self.z /= other.z;
    self.w /= other.w;
  }
}

impl<'a, T> DivAssign<&'a T> for Quat<T>
where
  T: DivAssign<T> + Copy,
{
  fn div_assign(&mut self, s: &'a T) {
    self.x /= *s;
    self.y /= *s;
    self.z /= *s;
    self.w /= *s;
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

  pub fn len() -> usize {
    4
  }

  pub fn to_tuple(self) -> (T, T, T, T) {
    (self.x, self.y, self.z, self.w)
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

  pub fn euler_xyz(euler: &Vec3<T>) -> Self {
    let p = (euler.x * T::half()).sin_cos();
    let h = (euler.y * T::half()).sin_cos();
    let b = (euler.z * T::half()).sin_cos();

    let sp = p.0;
    let sb = b.0;
    let sh = h.0;
    let cp = p.1;
    let cb = b.1;
    let ch = h.1;

    Self {
      w: cp * ch * cb + sp * sh * sb,
      x: sp * ch * cb - cp * sh * sb,
      y: cp * sh * cb + sp * ch * sb,
      z: cp * ch * sb - sp * sh * cb,
    }
  }

  pub fn euler_zxy(euler: &Vec3<T>) -> Self {
    let p = (euler.x * T::half()).sin_cos();
    let h = (euler.y * T::half()).sin_cos();
    let b = (euler.z * T::half()).sin_cos();

    let sp = p.0;
    let sb = b.0;
    let sh = h.0;
    let cp = p.1;
    let cb = b.1;
    let ch = h.1;

    Self {
      w: cp * ch * cb + sp * sh * sb,
      x: cp * sh * cb + sp * ch * sb,
      y: cp * ch * sb - sp * sh * cb,
      z: sp * ch * cb - cp * sh * sb,
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

  pub fn normalize(&self) -> Self {
    let mag_sq = self.length2();
    if mag_sq > T::zero() {
      let inv_sqrt = T::one() / mag_sq.sqrt();
      return *self * inv_sqrt;
    }
    *self
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

  pub fn conj(&self) -> Self {
    Self {
      x: -self.x,
      y: -self.y,
      z: -self.z,
      w: self.w,
    }
  }

  pub fn conjugate(&self) -> Self {
    Self {
      x: -self.x,
      y: -self.y,
      z: -self.z,
      w: self.w,
    }
  }

  pub fn inverse(&self) -> Self {
    self.conjugate()
  }
}

impl<T> Lerp<T> for Quat<T>
where
  T: Copy + num_traits::One + Mul<Output = T> + Add<Output = T> + Sub<Output = T>,
{
  #[inline(always)]
  fn lerp(self, b: Self, t: T) -> Self {
    self * (T::one() - t) + b * t
  }
}

impl<T> Slerp<T> for Quat<T>
where
  T: Scalar,
{
  fn slerp(self, other: Self, factor: T) -> Self {
    let dot = self.dot(other);

    let s = T::one() - factor;
    let t = if dot > T::zero() { factor } else { -factor };
    let q = self * s + other * t;

    q.normalize()
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
  T: num_traits::One,
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

impl<T> fmt::Display for Quat<T>
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

impl<T> From<Vec4<T>> for Quat<T>
where
  T: Copy + Div<Output = T>,
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
  T: Copy + Div<Output = T>,
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
  T: Copy + Div<Output = T>,
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
