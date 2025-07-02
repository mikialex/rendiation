use std::ops::{Add, Mul};

use facet::Facet;

use crate::*;

#[repr(C)]
#[rustfmt::skip]
#[derive(Serialize, Deserialize)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq, Facet)]
pub struct Mat2<T> {
  pub a1: T, pub a2: T,
  pub b1: T, pub b2: T,
}

impl<T> Mat2<T> {
  pub fn map<X: Clone>(self, f: impl Fn(T) -> X) -> Mat2<X> {
    let arr: [T; 4] = self.into();
    let arr = arr.map(f);
    arr.into()
  }
}

impl<T: Scalar> SquareMatrixDimension<1> for Mat2<T> {}
impl<T: Scalar> SquareMatrix<T> for Mat2<T> {
  fn identity() -> Self {
    Self::one()
  }
  fn transpose(&self) -> Self {
    let (a1, a2) = (self.a1, self.b1);
    let (b1, b2) = (self.a2, self.b2);
    #[rustfmt::skip]
    Mat2 {
      a1, a2,
      b1, b2,
    }
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
    Self {
      a1:  self.b2 * inv_det, a2: -self.b1 * inv_det,
      b1: -self.a2 * inv_det, b2: self.a1  * inv_det,
    }
    .into()
  }

  fn max_scale(&self) -> T {
    self.a1.sqrt()
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
  T: Copy,
{
  pub fn new(m11: T, m12: T, m21: T, m22: T) -> Self {
    Self {
      a1: m11,
      a2: m12,
      b1: m21,
      b2: m22,
    }
  }

  pub fn right(&self) -> Vec2<T> {
    Vec2::new(self.a1, self.a2)
  }

  pub fn up(&self) -> Vec2<T> {
    Vec2::new(self.b1, self.b2)
  }
}

impl<T> Mat2<T>
where
  T: Scalar,
{
  pub fn rotate_x(theta: T) -> Self {
    let (_s, c) = theta.sin_cos();
    Mat2::new(T::one(), T::zero(), T::zero(), c)
  }

  pub fn rotate_y(theta: T) -> Self {
    let (_s, c) = theta.sin_cos();
    Mat2::new(c, T::zero(), T::zero(), T::one())
  }

  pub fn rotate_z(theta: T) -> Self {
    let (s, c) = theta.sin_cos();
    Mat2::new(c, -s, s, c)
  }

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

impl<T> From<(T, T, T, T)> for Mat2<T>
where
  T: Copy,
{
  fn from(v: (T, T, T, T)) -> Self {
    #[rustfmt::skip]
    Self {
      a1: v.0, a2: v.1,
      b1: v.2, b2: v.3,
    }
  }
}

impl<T> AsRef<Mat2<T>> for Mat2<T> {
  fn as_ref(&self) -> &Mat2<T> {
    self
  }
}

impl<T> AsMut<Mat2<T>> for Mat2<T> {
  fn as_mut(&mut self) -> &mut Mat2<T> {
    self
  }
}
