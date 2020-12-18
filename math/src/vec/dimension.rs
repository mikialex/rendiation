use std::{marker::PhantomData, ops::*};

use num_traits::real::Real;

use crate::*;

pub enum Normalization {
  Unknown,
  Yes,
  No,
}

// this trait for avoid conflict impl
pub trait VectorImpl {}

// this trait for mark the vector's dimension
pub trait VectorDimension<const D: usize> {}

// this trait abstract for ops on vector
pub trait Vector<T: One + Zero>:
  Sized + Mul<T, Output = Self> + Sub<Self, Output = Self> + Add<Self, Output = Self> + Copy
{
  fn create<F>(f: F) -> Self
  where
    F: Fn() -> T;

  /// Perform the given operation on each field in the vector, returning a new point
  /// constructed from the operations.
  fn map<F>(self, f: F) -> Self
  where
    F: Fn(T) -> T;

  /// Construct a new vector where each component is the result of
  /// applying the given operation to each pair of components of the
  /// given vectors.
  fn zip<F>(self, v2: Self, f: F) -> Self
  where
    F: Fn(T, T) -> T;

  #[inline]
  fn one() -> Self {
    Self::create(|| T::one())
  }
  #[inline]
  fn zero() -> Self {
    Self::create(|| T::zero())
  }
}

pub trait RealVector<T: One + Zero + Real>: Vector<T> {
  #[inline]
  fn min(self, rhs: Self) -> Self {
    self.zip(rhs, |a, b| a.min(b))
  }
  #[inline]
  fn max(self, rhs: Self) -> Self {
    self.zip(rhs, |a, b| a.max(b))
  }
  #[inline]
  fn clamp(self, min: Self, max: Self) -> Self {
    self.min(min).max(max)
  }
  #[inline]
  fn saturate(self) -> Self {
    self.clamp(Self::zero(), Self::one())
  }
}

pub trait InnerProductSpace<T: One + Zero + Two + Real>: Vector<T> {
  #[inline]
  fn normalize(&self) -> Self {
    let mag_sq = self.length2();
    if mag_sq > T::zero() {
      let inv_sqrt = T::one() / mag_sq.sqrt();
      return *self * inv_sqrt;
    }
    *self
  }

  #[inline]
  fn reflect(&self, normal: Self) -> Self {
    *self - normal * self.dot(normal) * T::two()
  }

  #[inline]
  fn length(&self) -> T {
    self.length2().sqrt()
  }

  #[inline]
  fn length2(&self) -> T {
    self.dot(*self)
  }

  #[inline]
  fn distance(&self, b: Self) -> T {
    (*self - b).length()
  }

  fn dot(&self, b: Self) -> T;
}

impl<T, V> Lerp<T> for V
where
  T: Scalar,
  V: VectorImpl + Vector<T>,
{
  #[inline(always)]
  fn lerp(self, b: Self, t: T) -> Self {
    self * (T::one() - t) + b * t
  }
}

impl<T: Scalar, V> Slerp<T> for V
where
  T: Scalar,
  V: VectorImpl + InnerProductSpace<T>,
{
  fn slerp(self, other: Self, factor: T) -> Self {
    let dot = self.dot(other);

    let s = T::one() - factor;
    let t = if dot > T::zero() { factor } else { -factor };
    let q = self * s + other * t;

    q.normalize()
  }
}

pub trait DimensionalVec<T: Scalar, const D: usize> {
  type Type: Vector<T>
    + VectorDimension<D>
    + SpaceEntity<T, D>
    + RealVector<T>
    + InnerProductSpace<T>;
}

pub struct VectorMark<T>(PhantomData<T>);

impl<T: Scalar> DimensionalVec<T, 2> for VectorMark<T> {
  type Type = Vec2<T>;
}
impl<T: Scalar> DimensionalVec<T, 3> for VectorMark<T> {
  type Type = Vec3<T>;
}

impl<T: Scalar, const D: usize> DimensionalVec<T, D> for VectorMark<T> {
  default type Type = FakeHyperVec<T, D>;
}

pub type VectorType<T, const D: usize> = <VectorMark<T> as DimensionalVec<T, D>>::Type;
