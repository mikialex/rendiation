use std::ops::*;

use num_traits::real::Real;

use crate::*;

// this trait for avoid conflict impl
pub trait VectorImpl {}

// this trait for mark the vector's dimension
pub trait VectorDimension<const D: usize> {}

// this trait abstract for ops on vector
pub trait Vector<T: One + Zero + Copy>: Copy {
  fn create<F>(f: F) -> Self
  where
    F: Fn() -> T;

  /// Perform the given operation on each field in the vector, returning a new point
  /// constructed from the operations.
  #[must_use]
  fn map<F>(self, f: F) -> Self
  where
    F: Fn(T) -> T;

  /// Construct a new vector where each component is the result of
  /// applying the given operation to each pair of components of the
  /// given vectors.
  #[must_use]
  fn zip<F>(self, v2: Self, f: F) -> Self
  where
    F: Fn(T, T) -> T;

  #[inline]
  #[must_use]
  fn one() -> Self {
    Self::create(|| T::one())
  }
  #[inline]
  #[must_use]
  fn zero() -> Self {
    Self::create(|| T::zero())
  }
  #[inline]
  #[must_use]
  fn splat(v: T) -> Self {
    Self::create(|| v)
  }
}

/// the vector that in real number space
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

/// https://en.wikipedia.org/wiki/Vector_space
pub trait VectorSpace<T>:
  Add<Self, Output = Self>
  + Sub<Self, Output = Self>
  + Mul<T, Output = Self>
  + Div<T, Output = Self>
  + Sized
  + Copy
{
}

/// https://en.wikipedia.org/wiki/Inner_product
///
/// inner space define the length and angle based on vector space
pub trait InnerProductSpace<T: One + Zero + Two + Real + Copy>: VectorSpace<T> {
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
  fn reflect<Rhs: InnerData<Self>>(&self, normal: Rhs) -> Self {
    let normal = normal.get_inner();
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
  fn distance<Rhs: InnerData<Self>>(&self, b: Rhs) -> T {
    (*self - b.get_inner()).length()
  }

  #[inline]
  fn reverse(&self) -> Self {
    *self * -T::one()
  }

  #[inline]
  fn dot<Rhs: InnerData<Self>>(&self, b: Rhs) -> T {
    self.dot_impl(b.get_inner())
  }
  fn dot_impl(&self, b: Self) -> T;
}

impl<T, V> Lerp<T> for V
where
  T: Scalar,
  V: VectorImpl + VectorSpace<T>,
{
  #[inline(always)]
  fn lerp(self, b: Self, t: T) -> Self {
    self * (T::one() - t) + b * t
  }
}

impl<T: Scalar, V> Slerp<T> for V
where
  T: Scalar,
  V: VectorImpl + InnerProductSpace<T> + VectorSpace<T>,
{
  fn slerp(self, other: Self, factor: T) -> Self {
    let dot = self.dot(other);

    let s = T::one() - factor;
    let t = if dot > T::zero() { factor } else { -factor };
    let q = self * s + other * t;

    q.normalize()
  }
}
