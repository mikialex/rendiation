//! `N`-element vector.
use super::*;

pub mod common;
pub mod create;
#[cfg(feature = "mint")]
pub mod mint_impl;
pub mod operators;
#[cfg(feature = "serde")]
pub mod serde_impl;
pub mod swizzle;
pub use create::*;

/// `N`-element vector.
///
/// Vectors can be constructed from arrays of any type and size. There are
/// convenience constructor functions provided for the most common sizes.
///
/// ```
/// # use aljabar::*;
/// let a: Vector::<u32, 4> = vector!( 0u32, 1, 2, 3 );
/// assert_eq!(
///     a,
///     Vector::<u32, 4>::from([ 0u32, 1, 2, 3 ])
/// );
/// ```
#[repr(transparent)]
pub struct Vector<T, const N: usize>(pub(crate) [T; N]);

/// 2-element vector.
pub type Vector2<T> = Vector<T, 2>;
pub type Vec2<T> = Vector<T, 2>;

/// 3-element vector.
pub type Vector3<T> = Vector<T, 3>;
pub type Vec3<T> = Vector<T, 3>;

/// 4-element vector.
pub type Vector4<T> = Vector<T, 4>;
pub type Vec4<T> = Vector<T, 4>;

impl<T, const N: usize> Vector<T, { N }>
where
  T: Clone,
{
  /// Returns the first `M` elements of `self` in an appropriately sized
  /// `Vector`.
  ///
  /// Calling `first` with `M > N` is a compile error.
  pub fn first<const M: usize>(&self) -> Vector<T, { M }> {
    if M > N {
      panic!("attempt to return {} elements from a {}-vector", M, N);
    }
    let mut head = MaybeUninit::<Vector<T, { M }>>::uninit();
    let headp: *mut T = unsafe { mem::transmute(&mut head) };
    for i in 0..M {
      unsafe {
        headp.add(i).write(self[i].clone());
      }
    }
    unsafe { head.assume_init() }
  }

  /// Returns the last `M` elements of `self` in an appropriately sized
  /// `Vector`.
  ///
  /// Calling `last` with `M > N` is a compile error.
  pub fn last<const M: usize>(&self) -> Vector<T, { M }> {
    if M > N {
      panic!("attempt to return {} elements from a {}-vector", M, N);
    }
    let mut tail = MaybeUninit::<Vector<T, { M }>>::uninit();
    let tailp: *mut T = unsafe { mem::transmute(&mut tail) };
    for i in 0..M {
      unsafe {
        tailp.add(i + N - M).write(self[i].clone());
      }
    }
    unsafe { tail.assume_init() }
  }
}

impl<T> Vector3<T>
where
  T: Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T> + Clone,
{
  /// Return the cross product of the two vectors.
  pub fn cross(self, rhs: Vector3<T>) -> Self {
    let [x0, y0, z0]: [T; 3] = self.into();
    let [x1, y1, z1]: [T; 3] = rhs.into();
    Vector3::from([
      (y0.clone() * z1.clone()) - (z0.clone() * y1.clone()),
      (z0 * x1.clone()) - (x0.clone() * z1),
      (x0 * y1) - (y0 * x1),
    ])
  }
}

impl<T, const N: usize> Vector<T, { N }>
where
  T: Clone + PartialOrd,
{
  /// Return the largest value found in the vector, along with the
  /// associated index.
  pub fn argmax(&self) -> (usize, T) {
    let mut i_max = 0;
    let mut v_max = self.0[0].clone();
    for i in 1..N {
      if self.0[i] > v_max {
        i_max = i;
        v_max = self.0[i].clone();
      }
    }
    (i_max, v_max)
  }

  /// Return the largest value in the vector.
  pub fn max(&self) -> T {
    let mut v_max = self.0[0].clone();
    for i in 1..N {
      if self.0[i] > v_max {
        v_max = self.0[i].clone();
      }
    }
    v_max
  }

  /// Return the smallest value found in the vector, along with the
  /// associated index.
  pub fn argmin(&self) -> (usize, T) {
    let mut i_min = 0;
    let mut v_min = self.0[0].clone();
    for i in 1..N {
      if self.0[i] < v_min {
        i_min = i;
        v_min = self.0[i].clone();
      }
    }
    (i_min, v_min)
  }

  /// Return the smallest value in the vector.
  pub fn min(&self) -> T {
    let mut v_min = self.0[0].clone();
    for i in 1..N {
      if self.0[i] < v_min {
        v_min = self.0[i].clone();
      }
    }
    v_min
  }
}

// @EkardNT: The cool thing about this is that Rust apparently monomorphizes
// only those functions which are actually used. This means that this impl for
// vectors of any length N is able to support vectors of length N < 4. For
// example, calling x() on a Vector2 works, but attempting to call z() will
// result in a nice compile error.
//
// @maplant: Unfortunately, I think due to a compiler change this is no longer
// the case. I sure hope it's brought back, however...
impl<T, const N: usize> Vector<T, { N }>
where
  T: Clone,
{
  /// Alias for `.get(0).clone()`.
  ///
  /// # Panics
  /// When `N` = 0.
  pub fn x(&self) -> T {
    self.0[0].clone()
  }

  /// Alias for `.get(1).clone()`.
  ///
  /// # Panics
  /// When `N` < 2.
  pub fn y(&self) -> T {
    self.0[1].clone()
  }

  /// Alias for `.get(2).clone()`.
  ///
  /// # Panics
  /// When `N` < 3.
  pub fn z(&self) -> T {
    self.0[2].clone()
  }

  /// Alias for `.get(3).clone()`.
  ///
  /// # Panics
  /// When `N` < 4.
  pub fn w(&self) -> T {
    self.0[3].clone()
  }

  /// Alias for `.x()`.
  pub fn r(&self) -> T {
    self.x()
  }

  /// Alias for `.y()`.
  pub fn g(&self) -> T {
    self.y()
  }

  /// Alias for `.z()`.
  pub fn b(&self) -> T {
    self.z()
  }

  /// Alias for `.w()`.
  pub fn a(&self) -> T {
    self.w()
  }
}

impl<T, const N: usize> VectorSpace for Vector<T, { N }>
where
  T: Clone + Zero,
  T: Add<T, Output = T>,
  T: Sub<T, Output = T>,
  T: Mul<T, Output = T>,
  T: Div<T, Output = T>,
{
  type Scalar = T;
}

impl<T, const N: usize> MetricSpace for Vector<T, { N }>
where
  Self: InnerSpace,
{
  type Metric = <Self as VectorSpace>::Scalar;

  fn distance2(self, other: Self) -> Self::Metric {
    (other - self).magnitude2()
  }
}

impl<T, const N: usize> InnerSpace for Vector<T, { N }>
where
  T: Clone + Zero,
  T: Add<T, Output = T>,
  T: Sub<T, Output = T>,
  T: Mul<T, Output = T>,
  T: Div<T, Output = T>,
  // TODO: Remove this add assign bound. This is purely for ease of
  // implementation.
  T: AddAssign<T>,
  Self: Clone,
{
  fn dot(self, rhs: Self) -> T {
    let mut lhs = MaybeUninit::new(self);
    let mut rhs = MaybeUninit::new(rhs);
    let mut sum = <T as Zero>::zero();
    let lhsp: *mut MaybeUninit<T> = unsafe { mem::transmute(&mut lhs) };
    let rhsp: *mut MaybeUninit<T> = unsafe { mem::transmute(&mut rhs) };
    for i in 0..N {
      sum += unsafe {
        lhsp.add(i).replace(MaybeUninit::uninit()).assume_init()
          * rhsp.add(i).replace(MaybeUninit::uninit()).assume_init()
      };
    }
    sum
  }
}
