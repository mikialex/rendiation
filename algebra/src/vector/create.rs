use std::mem::{self, MaybeUninit};

use crate::{Matrix, Vector, Zero};

/// Constructs a new vector from an array. Necessary to help the compiler.
/// Prefer calling the macro `vector!`, which calls `new_vector` internally.
#[inline]
#[doc(hidden)]
pub fn new_vector<T, const N: usize>(elements: [T; N]) -> Vector<T, { N }> {
  Vector(elements)
}

/// Construct a new [Vector] of any size.
///
/// ```
/// # use aljabar::*;
/// let v: Vector<u32, 0> = vector![];
/// let v = vector![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
/// let v = vector![true, false, false, true];
/// ```
#[macro_export]
macro_rules! vector {
    ( $($elem:expr),* $(,)? ) => {
        $crate::new_vector([
            $($elem),*
        ])
    }
}

impl<T, const N: usize> From<[T; N]> for Vector<T, { N }> {
  fn from(array: [T; N]) -> Self {
    Vector::<T, { N }>(array)
  }
}

impl<T, const N: usize> From<Matrix<T, { N }, 1>> for Vector<T, { N }> {
  fn from(mat: Matrix<T, { N }, 1>) -> Self {
    let Matrix([v]) = mat;
    v
  }
}

impl<T, const N: usize> Zero for Vector<T, { N }>
where
  T: Zero,
{
  fn zero() -> Self {
    let mut origin = MaybeUninit::<Vector<T, { N }>>::uninit();
    let p: *mut T = unsafe { mem::transmute(&mut origin) };

    for i in 0..N {
      unsafe {
        p.add(i).write(<T as Zero>::zero());
      }
    }

    unsafe { origin.assume_init() }
  }

  fn is_zero(&self) -> bool {
    for i in 0..N {
      if !self.0[i].is_zero() {
        return false;
      }
    }
    true
  }
}

impl<T, const N: usize> Vector<T, { N }> {
  /// Constructs a new vector whose elements are equal to the value of the
  /// given function evaluated at the element's index.
  pub fn from_fn<Out, F>(mut f: F) -> Vector<Out, { N }>
  where
    F: FnMut(usize) -> Out,
  {
    let mut to = MaybeUninit::<Vector<Out, { N }>>::uninit();
    let top: *mut Out = unsafe { mem::transmute(&mut to) };
    for i in 0..N {
      unsafe { top.add(i).write(f(i)) }
    }
    unsafe { to.assume_init() }
  }
  /// Applies the given function to each element of the vector, constructing a
  /// new vector with the returned outputs.
  pub fn map<Out, F>(self, mut f: F) -> Vector<Out, { N }>
  where
    F: FnMut(T) -> Out,
  {
    self.indexed_map(|_, x: T| -> Out { f(x) })
  }

  pub fn indexed_map<Out, F>(self, mut f: F) -> Vector<Out, { N }>
  where
    F: FnMut(usize, T) -> Out,
  {
    let mut from = MaybeUninit::new(self);
    let mut to = MaybeUninit::<Vector<Out, { N }>>::uninit();
    let fromp: *mut MaybeUninit<T> = unsafe { mem::transmute(&mut from) };
    let top: *mut Out = unsafe { mem::transmute(&mut to) };
    for i in 0..N {
      unsafe {
        top.add(i).write(f(
          i,
          fromp.add(i).replace(MaybeUninit::uninit()).assume_init(),
        ));
      }
    }
    unsafe { to.assume_init() }
  }

  /// Converts the Vector into a Matrix with `N` columns each of size `1`.
  ///
  /// ```ignore
  /// # use aljabar::*;
  /// let v = vector!(1i32, 2, 3, 4);
  /// let m = Matrix::<i32, 1, 4>::from([
  ///     vector!(1i32),
  ///     vector!(2),
  ///     vector!(3),
  ///     vector!(4),
  /// ]);
  /// assert_eq!(v.transpose(), m);
  /// ```
  pub fn transpose(self) -> Matrix<T, 1, { N }> {
    let mut from = MaybeUninit::new(self);
    let mut st = MaybeUninit::<Matrix<T, 1, { N }>>::uninit();
    let fromp: *mut MaybeUninit<T> = unsafe { mem::transmute(&mut from) };
    let stp: *mut Vector<T, 1> = unsafe { mem::transmute(&mut st) };
    for i in 0..N {
      unsafe {
        stp.add(i).write(Vector::<T, 1>::from([fromp
          .add(i)
          .replace(MaybeUninit::uninit())
          .assume_init()]));
      }
    }
    unsafe { st.assume_init() }
  }

  /// Removes the last component and returns the vector with one fewer
  /// dimension.
  ///
  /// ```
  /// # use aljabar::*;
  /// let (xyz, w) = vector!(0u32, 1, 2, 3).truncate();
  /// assert_eq!(xyz, vector!(0u32, 1, 2));
  /// assert_eq!(w, 3);
  /// ```
  pub fn truncate(self) -> (Vector<T, { N - 1 }>, T) {
    let mut from = MaybeUninit::new(self);
    let mut head = MaybeUninit::<Vector<T, { N - 1 }>>::uninit();
    let fromp: *mut MaybeUninit<T> = unsafe { mem::transmute(&mut from) };
    let headp: *mut T = unsafe { mem::transmute(&mut head) };
    for i in 0..(N - 1) {
      unsafe {
        headp
          .add(i)
          .write(fromp.add(i).replace(MaybeUninit::uninit()).assume_init());
      }
    }
    (unsafe { head.assume_init() }, unsafe {
      fromp
        .add(N - 1)
        .replace(MaybeUninit::uninit())
        .assume_init()
    })
  }

  /// Extends the vector with an additional value.
  ///
  /// Useful for performing affine transformations.
  /// ```
  /// # use aljabar::*;
  /// let xyzw = vector!(0u32, 1, 2).extend(3);
  /// assert_eq!(xyzw, vector!(0u32, 1, 2, 3));
  /// ```
  pub fn extend(self, new: T) -> Vector<T, { N + 1 }> {
    let mut from = MaybeUninit::new(self);
    let mut head = MaybeUninit::<Vector<T, { N + 1 }>>::uninit();
    let fromp: *mut MaybeUninit<T> = unsafe { mem::transmute(&mut from) };
    let headp: *mut T = unsafe { mem::transmute(&mut head) };
    for i in 0..N {
      unsafe {
        headp
          .add(i)
          .write(fromp.add(i).replace(MaybeUninit::uninit()).assume_init());
      }
    }
    unsafe {
      headp.add(N).write(new);
      head.assume_init()
    }
  }
}
