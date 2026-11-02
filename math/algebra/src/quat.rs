use std::fmt::Debug;
use std::ops::{Add, Div, Mul, Neg, Sub};

use crate::*;

/// The quaternion `xi + yj + zk + w`, mainly used as the 3d rotation.
///
/// The rotation quaternion should be normalized, most of the rotation related methods assume
/// that. The memory layout is `x, y, z, w`, same as glam and gltf.
///
/// Note the `Default` is all zero, use [`Quat::identity`] for the identity rotation.
#[repr(C)]
#[derive(Serialize, Deserialize)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq, Facet)]
pub struct Quat<T> {
  pub x: T,
  pub y: T,
  pub z: T,
  pub w: T,
}
unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Quat<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Quat<T> {}

impl_as_ref_self!(Quat);

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

/// The Hamilton product, as rotation `a * b` applies `b` first.
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

impl<T> Quat<T> {
  /// Creates a new Quat from multiple components
  pub const fn new(x: T, y: T, z: T, w: T) -> Self {
    Self { x, y, z, w }
  }

  /// Map each component into another type.
  pub fn map<X>(self, f: impl Fn(T) -> X) -> Quat<X> {
    Quat {
      x: f(self.x),
      y: f(self.y),
      z: f(self.z),
      w: f(self.w),
    }
  }
}

impl<T> Quat<T>
where
  T: Scalar,
{
  /// The identity rotation.
  pub fn identity() -> Self {
    Self::new(T::zero(), T::zero(), T::zero(), T::one())
  }

  /// The rotation around the x axis.
  pub fn rotation_x(theta: T) -> Self {
    let theta_half = theta * T::half();

    Self {
      w: theta_half.cos(),
      x: theta_half.sin(),
      y: T::zero(),
      z: T::zero(),
    }
  }

  /// The rotation around the y axis.
  pub fn rotation_y(theta: T) -> Self {
    let theta_half = theta * T::half();

    Self {
      w: theta_half.cos(),
      x: T::zero(),
      y: theta_half.sin(),
      z: T::zero(),
    }
  }

  /// The rotation around the z axis.
  pub fn rotation_z(theta: T) -> Self {
    let theta_half = theta * T::half();

    Self {
      w: theta_half.cos(),
      x: T::zero(),
      y: T::zero(),
      z: theta_half.sin(),
    }
  }

  /// The rotation around the `axis`, the axis should be normalized.
  pub fn rotation(axis: Vec3<T>, theta: T) -> Self {
    let (s, c) = (theta * T::half()).sin_cos();

    Self {
      w: c,
      x: axis.x * s,
      y: axis.y * s,
      z: axis.z * s,
    }
  }

  /// The shortest arc rotation that rotates direction `a` to direction `b`, both of them should
  /// be normalized.
  ///
  /// If `a` and `b` are opposite, the rotation is 180 degrees around an arbitrary axis that is
  /// perpendicular to `a`.
  pub fn direction(a: &Vec3<T>, b: &Vec3<T>) -> Self {
    let axis = a.cross(*b);
    let cos_angle = a.dot(*b);

    let t0 = T::one() + cos_angle;
    if t0 <= T::eval::<{ scalar_transmute(1e-6) }>() {
      // pick the axis that least aligned with a to compute the perpendicular direction
      let other = if a.x.abs() < a.y.abs() && a.x.abs() < a.z.abs() {
        Vec3::new(T::one(), T::zero(), T::zero())
      } else if a.y.abs() < a.z.abs() {
        Vec3::new(T::zero(), T::one(), T::zero())
      } else {
        Vec3::new(T::zero(), T::zero(), T::one())
      };
      let axis = a.cross(other).normalize();
      return Self::new(axis.x, axis.y, axis.z, T::zero());
    }

    let t1 = (t0 + t0).sqrt().recip();
    let t2 = (t0 + t0) * t1 * T::half();

    Self {
      x: axis.x * t1,
      y: axis.y * t1,
      z: axis.z * t1,
      w: t2,
    }
  }

  /// The 4d dot product.
  pub fn dot(&self, b: Self) -> T {
    self.x * b.x + self.y * b.y + self.z * b.z + self.w * b.w
  }

  pub fn length2(&self) -> T {
    self.dot(*self)
  }

  pub fn length(&self) -> T {
    self.length2().sqrt()
  }

  /// The 4d euclidean distance, note it's not the rotation angle difference.
  pub fn distance(&self, b: Self) -> T {
    (*self - b).length()
  }

  /// Return the unit quaternion, the zero quaternion is normalized into the identity.
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

  /// The rotation axis of the unit quaternion, the x axis is returned if there is no rotation.
  pub fn axis(&self) -> Vec3<T> {
    let sin_theta_over2_sq = T::one() - self.w * self.w;
    if sin_theta_over2_sq <= T::zero() {
      return Vec3::new(T::one(), T::zero(), T::zero());
    }

    let v = Vec3::new(self.x, self.y, self.z);
    let inv_sqrt = T::one() / sin_theta_over2_sq.sqrt();

    v * Vec3::new(inv_sqrt, inv_sqrt, inv_sqrt)
  }

  /// The rotation angle of the unit quaternion in 0 to 2PI.
  pub fn angle(&self) -> T {
    // clamp to avoid nan caused by precision error
    self.w.max(-T::one()).min(T::one()).acos() * T::two()
  }

  /// The conjugate, which is the inverse rotation for the unit quaternion.
  #[must_use]
  pub fn conjugate(&self) -> Self {
    Self {
      x: -self.x,
      y: -self.y,
      z: -self.z,
      w: self.w,
    }
  }

  /// The inverse rotation, assume self is normalized so this is same as the conjugate.
  #[must_use]
  pub fn inverse(&self) -> Self {
    self.conjugate()
  }

  /// Rotate the vector, self should be normalized.
  pub fn rotate_vec(&self, v: Vec3<T>) -> Vec3<T> {
    // v + 2w(q x v) + 2q x (q x v)
    let q = Vec3::new(self.x, self.y, self.z);
    let t = q.cross(v) * T::two();
    v + t * self.w + q.cross(t)
  }
}

/// Convert the rotation matrix to quaternion, the matrix should be the pure rotation (orthonormal
/// with positive determinant).
impl<T: Scalar> From<Mat3<T>> for Quat<T> {
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

/// <http://www.euclideanspace.com/maths/algebra/realNormedAlgebra/quaternions/slerp/>
///
/// The input should be normalized, the output is normalized. The interpolation always takes the
/// shortest path.
impl<T> Slerp<T> for Quat<T>
where
  T: Scalar,
{
  fn slerp(self, target: Self, t: T) -> Self {
    if t == T::zero() {
      return self;
    } else if t == T::one() {
      return target;
    }

    let (x, y, z, w) = self.into();

    let mut cos_half_theta = self.dot(target);

    let mut result;
    if cos_half_theta < T::zero() {
      result = -target;
      cos_half_theta = -cos_half_theta;
    } else {
      result = target;
    }

    // if qa=qb or qa=-qb then theta = 0 and we can return qa
    if cos_half_theta >= T::one() {
      return self;
    }

    let sqr_sin_half_theta = T::one() - cos_half_theta * cos_half_theta;

    if sqr_sin_half_theta <= T::epsilon() {
      let s = T::one() - t;
      result.w = s * w + t * result.w;
      result.x = s * x + t * result.x;
      result.y = s * y + t * result.y;
      result.z = s * z + t * result.z;

      return result.normalize();
    }

    let sin_half_theta = sqr_sin_half_theta.sqrt();
    let half_theta = sin_half_theta.atan2(cos_half_theta);
    let ratio_a = ((T::one() - t) * half_theta).sin() / sin_half_theta;
    let ratio_b = (t * half_theta).sin() / sin_half_theta;

    result.w = w * ratio_a + result.w * ratio_b;
    result.x = x * ratio_a + result.x * ratio_b;
    result.y = y * ratio_a + result.y * ratio_b;
    result.z = z * ratio_a + result.z * ratio_b;

    result.normalize()
  }
}

impl<T> From<Vec4<T>> for Quat<T> {
  fn from(v: Vec4<T>) -> Self {
    Self {
      x: v.x,
      y: v.y,
      z: v.z,
      w: v.w,
    }
  }
}

impl<T> From<[T; 4]> for Quat<T> {
  fn from(v: [T; 4]) -> Self {
    let [x, y, z, w] = v;
    Self { x, y, z, w }
  }
}
impl<T> From<Quat<T>> for [T; 4] {
  fn from(q: Quat<T>) -> Self {
    [q.x, q.y, q.z, q.w]
  }
}
impl<T> From<Quat<T>> for Vec4<T> {
  fn from(q: Quat<T>) -> Self {
    Vec4::new(q.x, q.y, q.z, q.w)
  }
}

impl<T> From<(T, T, T, T)> for Quat<T> {
  fn from(v: (T, T, T, T)) -> Self {
    Self {
      x: v.0,
      y: v.1,
      z: v.2,
      w: v.3,
    }
  }
}

impl<T> From<Quat<T>> for (T, T, T, T) {
  fn from(q: Quat<T>) -> Self {
    (q.x, q.y, q.z, q.w)
  }
}
