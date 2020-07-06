use crate::*;
use std::ops::{Add, Div, Mul, Sub};

// https://github.com/maplant/aljabar/blob/master/src/lib.rs
pub trait MetricSpace: Sized {
  type Metric;

  /// Returns the distance squared between the two values.
  fn distance2(self, other: Self) -> Self::Metric;
}

/// Vectors that can be added together and multiplied by scalars form a
/// `VectorSpace`.
///
/// If a [Vector] implements [Add] and [Sub] and its scalar implements [Mul] and
/// [Div], then that vector is part of a `VectorSpace`.
pub trait VectorSpace
where
  Self: Sized + Clone + Zero,
  Self: Add<Self, Output = Self>,
  Self: Sub<Self, Output = Self>,
  Self: Mul<<Self as VectorSpace>::Scalar, Output = Self>,
  Self: Div<<Self as VectorSpace>::Scalar, Output = Self>,
{
  // I only need Div, but I felt like I had to add them all...
  type Scalar: Add<Self::Scalar, Output = Self::Scalar>
    + Sub<Self::Scalar, Output = Self::Scalar>
    + Mul<Self::Scalar, Output = Self::Scalar>
    + Div<Self::Scalar, Output = Self::Scalar>;

  /// Linear interpolate between the two vectors with a weight of `t`.
  fn lerp(self, other: Self, t: Self::Scalar) -> Self {
    self.clone() + ((other - self) * t)
  }
}

/// Vector spaces that have an inner (also known as "dot") product.
pub trait InnerSpace: VectorSpace
where
  Self: Clone,
  Self: MetricSpace<Metric = <Self as VectorSpace>::Scalar>,
{
  /// Return the inner (also known as "dot") product.
  fn dot(self, other: Self) -> Self::Scalar;

  /// Returns the squared length of the value.
  fn magnitude2(self) -> Self::Scalar {
    self.clone().dot(self)
  }

  /// Returns the [reflection](https://en.wikipedia.org/wiki/Reflection_(mathematics))
  /// of the current vector with respect to the given surface normal. The
  /// surface normal must be of length 1 for the return value to be
  /// correct. The current vector is interpreted as pointing toward the
  /// surface, and does not need to be normalized.
  fn reflect(self, surface_normal: Self) -> Self {
    let a = surface_normal.clone() * self.clone().dot(surface_normal);
    self - (a.clone() + a)
  }
}

impl<T: Arithmetic> VectorSpace for Vec3<T> {
  type Scalar = T;
}

impl<T: Arithmetic> MetricSpace for Vec3<T> {
  type Metric = T;

  fn distance2(self, _other: Self) -> Self::Metric{
    todo!()
    // self.dot(self);
  }
}

impl<T: Arithmetic> InnerSpace for Vec3<T> {
  fn dot(self, _other: Self) -> Self::Scalar{
    todo!()
    // self.dot(self);
  }
}

