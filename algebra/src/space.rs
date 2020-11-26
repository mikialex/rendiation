use crate::*;

/// Defines the additive identity for `Self`.
pub trait Zero {
  /// Returns the additive identity of `Self`.
  fn zero() -> Self;

  /// Returns true if the value is the additive identity.
  fn is_zero(&self) -> bool;
}

macro_rules! impl_zero {
  // Default $zero to '0' if not provided.
  (
          $type:ty
      ) => {
    impl_zero! { $type, 0 }
  };
  // Main impl.
  (
          $type:ty,
          $zero:expr
      ) => {
    impl Zero for $type {
      fn zero() -> Self {
        $zero
      }

      fn is_zero(&self) -> bool {
        *self == $zero
      }
    }
  };
}

impl_zero! { bool, false }
impl_zero! { f32, 0.0 }
impl_zero! { f64, 0.0 }
impl_zero! { i8 }
impl_zero! { i16 }
impl_zero! { i32 }
impl_zero! { i64 }
impl_zero! { i128 }
impl_zero! { isize }
impl_zero! { u8 }
impl_zero! { u16 }
impl_zero! { u32 }
impl_zero! { u64 }
impl_zero! { u128 }
impl_zero! { usize }

/// Defines the multiplicative identity element for `Self`.
///
/// For Matrices, `one` is an alias for the unit matrix.
pub trait One {
  /// Returns the multiplicative identity for `Self`.
  fn one() -> Self;

  /// Returns true if the value is the multiplicative identity.
  fn is_one(&self) -> bool;
}

macro_rules! impl_one {
  // Default $one to '1' if not provided.
  (
          $type:ty
      ) => {
    impl_one! { $type, 1 }
  };
  // Main impl.
  (
          $type:ty,
          $one:expr
      ) => {
    impl One for $type {
      fn one() -> Self {
        $one
      }

      fn is_one(&self) -> bool {
        *self == $one
      }
    }
  };
}

impl_one! { bool, true }
impl_one! { f32, 1.0 }
impl_one! { f64, 1.0 }
impl_one! { i8 }
impl_one! { i16 }
impl_one! { i32 }
impl_one! { i64 }
impl_one! { i128 }
impl_one! { isize }
impl_one! { u8 }
impl_one! { u16 }
impl_one! { u32 }
impl_one! { u64 }
impl_one! { u128 }
impl_one! { usize }

/// Values that are [real numbers](https://en.wikipedia.org/wiki/Real_number#Axiomatic_approach).
pub trait Real
where
  Self: Sized,
  Self: Add<Output = Self>,
  Self: Sub<Output = Self>,
  Self: Mul<Output = Self>,
  Self: Div<Output = Self>,
  Self: Neg<Output = Self>,
{
  fn sqrt(self) -> Self;

  fn mul2(self) -> Self;

  fn div2(self) -> Self;

  fn abs(self) -> Self;

  /// Returns the sine of the angle.
  fn sin(self) -> Self;

  /// Returns the cosine of the angle.
  fn cos(self) -> Self;

  /// Returns the tangent of the angle.
  fn tan(self) -> Self;

  /// Returns the four quadrant arctangent of `self` and `x` in radians.
  fn atan2(self, x: Self) -> Self;

  /// Returns the sine and the cosine of the angle.
  fn sin_cos(self) -> (Self, Self);
}

impl Real for f32 {
  fn sqrt(self) -> Self {
    self.sqrt()
  }

  fn mul2(self) -> Self {
    2.0 * self
  }

  fn div2(self) -> Self {
    self / 2.0
  }

  fn abs(self) -> Self {
    self.abs()
  }

  fn sin(self) -> Self {
    self.sin()
  }

  fn cos(self) -> Self {
    self.cos()
  }

  fn tan(self) -> Self {
    self.tan()
  }

  fn atan2(self, x: Self) -> Self {
    self.atan2(x)
  }

  fn sin_cos(self) -> (Self, Self) {
    (self.sin(), self.cos())
  }
}

impl Real for f64 {
  fn sqrt(self) -> Self {
    self.sqrt()
  }

  fn mul2(self) -> Self {
    2.0 * self
  }

  fn div2(self) -> Self {
    self / 2.0
  }

  fn abs(self) -> Self {
    self.abs()
  }

  fn sin(self) -> Self {
    self.sin()
  }

  fn cos(self) -> Self {
    self.cos()
  }

  fn tan(self) -> Self {
    self.tan()
  }

  fn atan2(self, x: Self) -> Self {
    self.atan2(x)
  }

  fn sin_cos(self) -> (Self, Self) {
    (self.sin(), self.cos())
  }
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

/// A type with a distance function between two values.
pub trait MetricSpace: Sized {
  type Metric;

  /// Returns the distance squared between the two values.
  fn distance2(self, other: Self) -> Self::Metric;
}

/// A [MetricSpace] where the metric is a real number.
pub trait RealMetricSpace: MetricSpace
where
  Self::Metric: Real,
{
  /// Returns the distance between the two values.
  fn distance(self, other: Self) -> Self::Metric {
    self.distance2(other).sqrt()
  }
}

impl<T> RealMetricSpace for T
where
  T: MetricSpace,
  <T as MetricSpace>::Metric: Real,
{
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

/// Defines an [InnerSpace] where the Scalar is a real number. Automatically
/// implemented.
pub trait RealInnerSpace: InnerSpace
where
  Self: Clone,
  Self: MetricSpace<Metric = <Self as VectorSpace>::Scalar>,
  <Self as VectorSpace>::Scalar: Real,
{
  /// Returns the length of the vector.
  fn magnitude(self) -> Self::Scalar {
    self.clone().dot(self).sqrt()
  }

  /// Returns a vector with the same direction and a magnitude of `1`.
  fn normalize(self) -> Self
  where
    Self::Scalar: One,
  {
    self.normalize_to(<Self::Scalar as One>::one())
  }

  /// Returns a vector with the same direction and a given magnitude.
  fn normalize_to(self, magnitude: Self::Scalar) -> Self {
    self.clone() * (magnitude / self.magnitude())
  }

  /// Returns the
  /// [vector projection](https://en.wikipedia.org/wiki/Vector_projection)
  /// of the current inner space projected onto the supplied argument.
  fn project_on(self, other: Self) -> Self {
    other.clone() * (self.dot(other.clone()) / other.magnitude2())
  }
}

impl<T> RealInnerSpace for T
where
  T: InnerSpace,
  <T as VectorSpace>::Scalar: Real,
{
}
