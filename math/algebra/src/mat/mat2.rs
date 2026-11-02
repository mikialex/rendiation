use std::ops::{Add, Mul};

use crate::*;

/// The column major 2x2 matrix, `a` is the first column and `b` is the second column.
///
/// It is the linear transform in 2d space, or the "sub" linear part of [`Mat3`].
#[repr(C)]
#[rustfmt::skip]
#[derive(Serialize, Deserialize)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq, Facet)]
pub struct Mat2<T> {
  pub a1: T, pub a2: T,
  pub b1: T, pub b2: T,
}

impl<T> Mat2<T> {
  /// Create the matrix column by column: `m11, m12` is the first column.
  pub const fn new(m11: T, m12: T, m21: T, m22: T) -> Self {
    Self {
      a1: m11,
      a2: m12,
      b1: m21,
      b2: m22,
    }
  }

  /// Map each element into another type.
  pub fn map<X>(self, f: impl Fn(T) -> X) -> Mat2<X> {
    let arr: [T; 4] = self.into();
    arr.map(f).into()
  }
}

impl<T: Copy> Mat2<T> {
  /// The first column, the transformed x axis.
  pub fn right(&self) -> Vec2<T> {
    Vec2::new(self.a1, self.a2)
  }

  /// The second column, the transformed y axis.
  pub fn up(&self) -> Vec2<T> {
    Vec2::new(self.b1, self.b2)
  }
}

impl<T: Scalar> SquareMatrixDimension<1> for Mat2<T> {}
impl<T: Scalar> SquareMatrix<T> for Mat2<T> {
  fn identity() -> Self {
    Self::one()
  }
  fn transpose(&self) -> Self {
    #[rustfmt::skip]
    Mat2::new(
      self.a1, self.b1,
      self.a2, self.b2,
    )
  }
  fn det(&self) -> T {
    self.a1 * self.b2 - self.a2 * self.b1
  }
  fn inverse(&self) -> Option<Self> {
    let det = self.det();
    if det == T::zero() {
      return None;
    }
    let inv_det = T::one() / det;
    #[rustfmt::skip]
    Mat2::new(
       self.b2 * inv_det, -self.a2 * inv_det,
      -self.b1 * inv_det,  self.a1 * inv_det,
    )
    .into()
  }

  fn max_scale(&self) -> T {
    let x = self.a1 * self.a1 + self.a2 * self.a2;
    let y = self.b1 * self.b1 + self.b2 * self.b2;
    x.max(y).sqrt()
  }
}

unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for Mat2<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for Mat2<T> {}

impl<T> Mul for Mat2<T>
where
  T: Copy + Mul<Output = T> + Add<Output = T>,
{
  type Output = Self;

  fn mul(self, b: Self) -> Self {
    let a = self;

    Mat2 {
      a1: a.a1 * b.a1 + a.b1 * b.a2,
      a2: a.a2 * b.a1 + a.b2 * b.a2,
      b1: a.a1 * b.b1 + a.b1 * b.b2,
      b2: a.a2 * b.b1 + a.b2 * b.b2,
    }
  }
}

impl<T> Mul<Vec2<T>> for Mat2<T>
where
  T: Copy + Add<Output = T> + Mul<Output = T>,
{
  type Output = Vec2<T>;

  fn mul(self, v: Vec2<T>) -> Vec2<T> {
    Vec2 {
      x: v.x * self.a1 + v.y * self.b1,
      y: v.x * self.a2 + v.y * self.b2,
    }
  }
}

impl<T> Mat2<T>
where
  T: Scalar,
{
  /// The xy part of the 3d rotation around the x axis (the 3d rotation projected to xy plane).
  pub fn rotate_x(theta: T) -> Self {
    let c = theta.cos();
    Mat2::new(T::one(), T::zero(), T::zero(), c)
  }

  /// The xy part of the 3d rotation around the y axis (the 3d rotation projected to xy plane).
  pub fn rotate_y(theta: T) -> Self {
    let c = theta.cos();
    Mat2::new(c, T::zero(), T::zero(), T::one())
  }

  /// The 2d counter clockwise rotation, same as the 3d rotation around the z axis.
  pub fn rotate_z(theta: T) -> Self {
    let (s, c) = theta.sin_cos();
    Mat2::new(c, s, -s, c)
  }

  /// The xy part of the 3d rotation around the normalized `axis`, see [`Mat3::rotate`].
  pub fn rotate(axis: Vec3<T>, theta: T) -> Self {
    let (s, c) = theta.sin_cos();

    let x = axis.x;
    let y = axis.y;
    let z = axis.z;

    let t = T::one() - c;
    let tx = t * x;
    let ty = t * y;

    let a1 = tx * x + c;
    let a2 = tx * y + s * z;

    let b1 = tx * y - s * z;
    let b2 = ty * y + c;

    #[rustfmt::skip]
    Mat2 {
      a1, a2,
      b1, b2,
    }
  }

  /// The non uniform scale.
  pub fn scale(x: T, y: T) -> Self {
    #[rustfmt::skip]
    Mat2 {
      a1: x,         a2: T::zero(),
      b1: T::zero(), b2: y,
    }
  }
}

impl<T> num_traits::Zero for Mat2<T>
where
  T: num_traits::Zero + Copy + PartialEq,
{
  #[inline(always)]
  fn zero() -> Self {
    #[rustfmt::skip]
    Mat2 {
      a1: T::zero(), a2: T::zero(),
      b1: T::zero(), b2: T::zero(),
    }
  }
  #[inline(always)]
  fn is_zero(&self) -> bool {
    self.eq(&Self::zero())
  }
}

impl<T> num_traits::One for Mat2<T>
where
  T: num_traits::One + num_traits::Zero + Copy,
{
  #[inline(always)]
  fn one() -> Self {
    #[rustfmt::skip]
    Mat2 {
      a1: T::one(),  a2: T::zero(),
      b1: T::zero(), b2: T::one(),
    }
  }
}

impl<T> From<(T, T, T, T)> for Mat2<T> {
  fn from(v: (T, T, T, T)) -> Self {
    let (a1, a2, b1, b2) = v;
    Self { a1, a2, b1, b2 }
  }
}
