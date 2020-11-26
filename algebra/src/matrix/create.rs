use crate::*;

/// Constructs a new matrix from an array, using the more visually natural row
/// major order. Necessary to help the compiler. Prefer calling the macro
/// `matrix!`, which calls `new_matrix` internally.
#[inline]
#[doc(hidden)]
pub fn new_matrix<T: Clone, const N: usize, const M: usize>(
  rows: [[T; M]; N],
) -> Matrix<T, { N }, { M }> {
  Matrix::<T, { M }, { N }>::from(rows).transpose()
}

/// Construct a [Matrix] of any size. The matrix is specified in row-major
/// order, but this function converts it to aljabar's native column-major order.
///
/// ```ignore
/// # use aljabar::*;
/// // `matrix` allows you to create a matrix using natural writing order (row-major).
/// let m1: Matrix<u32, 4, 3> = matrix![
///     [0, 1, 2],
///     [3, 4, 5],
///     [6, 7, 8],
///     [9, 0, 1],
/// ];
///
/// // The equivalent code using the From implementation is below. Note the From
/// // usage requires you to specify the entries in column-major order, and create
/// // the sub-Vectors explicitly.
/// let m2: Matrix<u32, 4, 3> = Matrix::<u32, 4, 3>::from([
///     Vector::<u32, 4>::from([0, 3, 6, 9]),
///     Vector::<u32, 4>::from([1, 4, 7, 0]),
///     Vector::<u32, 4>::from([2, 5, 8, 1]),
/// ]);
///
/// assert_eq!(m1, m2);
/// ```
#[macro_export]
macro_rules! matrix {
    ( $item:expr ) => {
     $crate::new_matrix([
            [ $item ]
        ])
    };

    ( $($rows:expr),* $(,)? ) => {
        $crate::new_matrix([
            $($rows),*
        ])
    };
}

impl<T, const N: usize, const M: usize> Zero for Matrix<T, { N }, { M }>
where
  T: Zero,
  // This bound is a consequence of the previous, but I'm going to preemptively
  // help out the compiler a bit on this one.
  Vector<T, { N }>: Zero,
{
  fn zero() -> Self {
    let mut zero_mat = MaybeUninit::<[Vector<T, { N }>; M]>::uninit();
    let matp: *mut Vector<T, { N }> = unsafe { mem::transmute(&mut zero_mat) };

    for i in 0..M {
      unsafe {
        matp.add(i).write(Vector::<T, { N }>::zero());
      }
    }

    Matrix::<T, { N }, { M }>(unsafe { zero_mat.assume_init() })
  }

  fn is_zero(&self) -> bool {
    for i in 0..M {
      if !self.0[i].is_zero() {
        return false;
      }
    }
    true
  }
}

/// Constructs a unit matrix.
impl<T, const N: usize> One for Matrix<T, { N }, { N }>
where
  T: Zero + One + Clone,
  Self: PartialEq<Self>,
{
  fn one() -> Self {
    let mut unit_mat = MaybeUninit::<[Vector<T, { N }>; N]>::uninit();
    let matp: *mut Vector<T, { N }> = unsafe { mem::transmute(&mut unit_mat) };
    for i in 0..N {
      let mut unit_vec = MaybeUninit::<Vector<T, { N }>>::uninit();
      let vecp: *mut T = unsafe { mem::transmute(&mut unit_vec) };
      for j in 0..i {
        unsafe {
          vecp.add(j).write(<T as Zero>::zero());
        }
      }
      unsafe {
        vecp.add(i).write(<T as One>::one());
      }
      for j in (i + 1)..N {
        unsafe {
          vecp.add(j).write(<T as Zero>::zero());
        }
      }
      unsafe {
        matp.add(i).write(unit_vec.assume_init());
      }
    }
    Matrix::<T, { N }, { N }>(unsafe { unit_mat.assume_init() })
  }

  fn is_one(&self) -> bool {
    self == &<Self as One>::one()
  }
}
