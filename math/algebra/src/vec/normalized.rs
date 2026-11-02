use std::{fmt::Debug, marker::PhantomData, ops::*};

use crate::*;

pub type NormalizedVec2<T> = NormalizedVector<T, Vec2<T>>;
pub type NormalizedVec3<T> = NormalizedVector<T, Vec3<T>>;
pub type NormalizedVec4<T> = NormalizedVector<T, Vec4<T>>;

/// The wrapper that guarantees (by construction) the inner vector is normalized, to avoid
/// unnecessary normalize and to express the requirement in api.
///
/// The arithmetic operations on it return the unwrapped vector because the result may be not
/// normalized anymore. Mutating the inner value by `DerefMut` is the caller's responsibility to
/// keep it normalized.
#[repr(transparent)]
#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Facet)]
pub struct NormalizedVector<T, V> {
  pub value: V,
  phantom: PhantomData<T>,
}

impl<T, V: Debug> Debug for NormalizedVector<T, V> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("NormalizedVector")
      .field("value", &self.value)
      .finish()
  }
}

/// Convert a vector into [`NormalizedVector`].
pub trait IntoNormalizedVector<T, V> {
  /// Normalize and wrap, the zero length vector is kept as is.
  #[must_use]
  fn into_normalized(self) -> NormalizedVector<T, V>;

  /// # Safety
  ///
  /// self must be normalized
  #[must_use]
  unsafe fn into_normalized_unchecked(self) -> NormalizedVector<T, V>;
}

impl<T: Scalar, V: InnerProductSpace<T>> IntoNormalizedVector<T, V> for V {
  #[inline(always)]
  fn into_normalized(self) -> NormalizedVector<T, V> {
    unsafe { NormalizedVector::wrap(self.normalize()) }
  }
  #[inline(always)]
  unsafe fn into_normalized_unchecked(self) -> NormalizedVector<T, V> {
    unsafe { NormalizedVector::wrap(self) }
  }
}

impl<T, V> NormalizedVector<T, V> {
  /// # Safety
  ///
  /// self must be normalized
  #[inline(always)]
  pub unsafe fn wrap(v: V) -> Self {
    Self {
      value: v,
      phantom: PhantomData,
    }
  }
}

impl<T: Scalar, V: InnerProductSpace<T>> NormalizedVector<T, V> {
  /// Already normalized, this is a no op.
  #[inline]
  #[must_use]
  pub fn normalize(&self) -> Self {
    *self
  }

  /// self should be normalized
  ///
  /// of course input normal should also be normalized
  #[inline]
  #[must_use]
  pub fn reflect(&self, normal: Self) -> Self {
    unsafe { self.value.reflect(*normal).into_normalized_unchecked() }
  }

  #[inline]
  pub fn length(&self) -> T {
    T::one()
  }

  #[inline]
  pub fn length2(&self) -> T {
    T::one()
  }

  #[inline]
  #[must_use]
  pub fn reverse(&self) -> Self {
    unsafe { self.value.reverse().into_normalized_unchecked() }
  }
}

/// Get the inner vector, implemented for both the vector and the [`NormalizedVector`], so the api
/// can accept both of them.
pub trait InnerData<T> {
  fn get_inner(self) -> T;
}

impl<T, V> InnerData<V> for NormalizedVector<T, V> {
  #[inline(always)]
  fn get_inner(self) -> V {
    self.value
  }
}
impl<V> InnerData<V> for V {
  #[inline(always)]
  fn get_inner(self) -> V {
    self
  }
}

// after add / sub / mul scalar, the vector may not be normalized
impl<T, V: VectorSpace<T>, Rhs: InnerData<V>> Add<Rhs> for NormalizedVector<T, V> {
  type Output = V;
  #[inline(always)]
  fn add(self, rhs: Rhs) -> Self::Output {
    self.value + rhs.get_inner()
  }
}
impl<T, V: VectorSpace<T>, Rhs: InnerData<V>> Sub<Rhs> for NormalizedVector<T, V> {
  type Output = V;
  #[inline(always)]
  fn sub(self, rhs: Rhs) -> Self::Output {
    self.value - rhs.get_inner()
  }
}
impl<T, V: VectorSpace<T>> Mul<T> for NormalizedVector<T, V> {
  type Output = V;
  #[inline(always)]
  fn mul(self, rhs: T) -> Self::Output {
    self.value * rhs
  }
}
impl<T, V: VectorSpace<T>> Div<T> for NormalizedVector<T, V> {
  type Output = V;
  #[inline(always)]
  fn div(self, rhs: T) -> Self::Output {
    self.value / rhs
  }
}

impl<T, V> Deref for NormalizedVector<T, V> {
  type Target = V;
  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    &self.value
  }
}
impl<T, V> DerefMut for NormalizedVector<T, V> {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.value
  }
}

#[test]
fn test() {
  let a = Vec3::new(1., 1., 1.).into_normalized();
  let a = a.normalize(); // should use cheaper method
  assert!((a.value.length() - 1.0_f32).abs() < 1e-6);
  assert_eq!(a.length(), 1.0);
  assert_eq!(a.length2(), 1.0);

  let b = Vec3::new(1., 1., 1.);
  let c: Vec3<f32> = *a + b;
  assert_eq!(c, a + b);
  let c = a + a;
  assert_eq!(c, a.value * 2.);
  let nc = c.normalize(); // ra code jump is misleading, but it actually used correct impl
  assert!((nc - a.value).length() < 1e-6);

  assert_eq!(a.reverse().value, -a.value);
  assert_eq!(a * 2., a.value * 2.);
  assert_eq!(a / 2., a.value / 2.);
  assert_eq!(a - b, a.value - b);

  let r = Vec3::new(1., -1., 0.).into_normalized();
  let n = Vec3::new(0., 1., 0.).into_normalized();
  let reflected = r.reflect(n);
  assert!((reflected.value - Vec3::new(1., 1., 0.).normalize()).length() < 1e-6);

  // zero vector is kept as is
  let z = Vec3::<f32>::zero().into_normalized();
  assert_eq!(z.value, Vec3::zero());
}
