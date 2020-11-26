/// An `N`-by-`M` Column Major matrix.
mod common;
mod create;
mod lu;
mod mint_impl;
mod operators;
mod serde_impl;

pub use create::*;
pub use lu::*;

/// An `N`-by-`M` Column Major matrix.
///
/// Matrices can be created from arrays of Vectors of any size and scalar type.
/// As with Vectors there are convenience constructor functions for square
/// matrices of the most common sizes.
///
/// ```ignore
/// # use aljabar::*;
/// let a = Matrix::<f32, 3, 3>::from( [ vector!( 1.0, 0.0, 0.0 ),
///                                      vector!( 0.0, 1.0, 0.0 ),
///                                      vector!( 0.0, 0.0, 1.0 ), ] );
/// let b: Matrix::<i32, 3, 3> = matrix![
///     [ 0, -3, 5 ],
///     [ 6, 1, -4 ],
///     [ 2, 3, -2 ]
/// ];
/// ```
///
/// All operations performed on matrices produce fixed-size outputs. For
/// example, taking the `transpose` of a non-square matrix will produce a matrix
/// with the width and height swapped:
///
/// ```ignore
/// # use aljabar::*;
/// assert_eq!(
///     Matrix::<i32, 1, 2>::from( [ vector!( 1 ), vector!( 2 ) ] )
///         .transpose(),
///     Matrix::<i32, 2, 1>::from( [ vector!( 1, 2 ) ] )
/// );
/// ```
///
/// # Indexing
///
/// Matrices can be indexed by either their native column major storage or by
/// the more natural row major method. In order to use row-major indexing, call
/// `.index` or `.index_mut` on the matrix with a pair of indices. Calling
/// `.index` with a single index will produce a vector representing the
/// appropriate column of the matrix.
///
/// ```
/// # use aljabar::*;
/// let m: Matrix::<i32, 2, 2> = matrix![
///     [ 0, 2 ],
///     [ 1, 3 ],
/// ];
///
/// // Column-major indexing:
/// assert_eq!(m[0][0], 0);
/// assert_eq!(m[0][1], 1);
/// assert_eq!(m[1][0], 2);
/// assert_eq!(m[1][1], 3);
///
/// // Row-major indexing:
/// assert_eq!(m[(0, 0)], 0);
/// assert_eq!(m[(1, 0)], 1);
/// assert_eq!(m[(0, 1)], 2);
/// assert_eq!(m[(1, 1)], 3);
/// ```
///
/// # Iterating
///
/// Matrices are iterated most naturally over their columns, for which the
/// following three functions are provided:
///
/// * [column_iter](Matrix::column_iter), for immutably iterating over columns.
/// * [column_iter_mut](Matrix::column_iter_mut), for mutably iterating over
///   columns.
/// * [into_iter](IntoIterator::into_iter), for taking ownership of the columns.
///
/// Matrices can also be iterated over by their rows, however they can only
/// be iterated over by [RowViews](RowView), as they are not the natural
/// storage for Matrices. The following functions are provided:
///
/// * [row_iter](Matrix::row_iter), for immutably iterating over row views.
/// * [row_iter_mut](Matrix::row_iter_mut), for mutably iterating over row views
///   ([RowViewMut]).
/// * In order to take ownership of the rows of the matrix, `into_iter` should
///   called on the result of a [transpose](Matrix::transpose).
#[repr(transparent)]
pub struct Matrix<T, const N: usize, const M: usize>(pub(crate) [Vector<T, { N }>; M]);

/// A 2-by-2 square matrix.
pub type Matrix2<T> = Matrix<T, 2, 2>;
pub type Mat2<T> = Matrix<T, 2, 2>;

/// A 3-by-3 square matrix.
pub type Matrix3<T> = Matrix<T, 3, 3>;
pub type Mat3<T> = Matrix<T, 3, 3>;

/// A 4-by-4 square matrix.
pub type Matrix4<T> = Matrix<T, 4, 4>;
pub type Mat4<T> = Matrix<T, 4, 4>;

impl<T, const N: usize, const M: usize> Matrix<T, { N }, { M }> {
  /// Swap the two given columns in-place.
  pub fn swap_columns(&mut self, a: usize, b: usize) {
    unsafe { core::ptr::swap(&mut self.0[a], &mut self.0[b]) };
  }

  /// Swap the two given rows in-place.
  pub fn swap_rows(&mut self, a: usize, b: usize) {
    for v in self.0.iter_mut() {
      unsafe { core::ptr::swap(&mut v[a], &mut v[b]) };
    }
  }

  /// Swap the two given elements at index `a` and index `b`.
  ///
  /// The indices are expressed in the form `(column, row)`, which may be
  /// confusing given the indexing strategy for matrices.
  pub fn swap_elements(&mut self, (acol, arow): (usize, usize), (bcol, brow): (usize, usize)) {
    unsafe { core::ptr::swap(&mut self[acol][arow], &mut self[bcol][brow]) };
  }

  /// Returns an immutable iterator over the columns of the matrix.
  pub fn column_iter<'a>(&'a self) -> impl Iterator<Item = &'a Vector<T, { N }>> {
    self.0.iter()
  }

  /// Returns a mutable iterator over the columns of the matrix.
  pub fn column_iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut Vector<T, { N }>> {
    self.0.iter_mut()
  }

  /// Returns an immutable iterator over the rows of the matrix.
  pub fn row_iter<'a>(&'a self) -> impl Iterator<Item = RowView<'a, T, { N }, { M }>> {
    RowIter {
      row: 0,
      matrix: self,
    }
  }

  /// Returns a mutable iterator over the rows of the matrix
  pub fn row_iter_mut<'a>(&'a mut self) -> impl Iterator<Item = RowViewMut<'a, T, { N }, { M }>> {
    RowIterMut {
      row: 0,
      matrix: self,
      phantom: PhantomData,
    }
  }

  /// Applies the given function to each element of the matrix, constructing a
  /// new matrix with the returned outputs.
  pub fn map<Out, F>(self, mut f: F) -> Matrix<Out, { N }, { M }>
  where
    F: FnMut(T) -> Out,
  {
    let mut from = MaybeUninit::new(self);
    let mut to = MaybeUninit::<Matrix<Out, { N }, { M }>>::uninit();
    let fromp: *mut MaybeUninit<Vector<T, { N }>> = unsafe { mem::transmute(&mut from) };
    let top: *mut Vector<Out, { N }> = unsafe { mem::transmute(&mut to) };
    for i in 0..M {
      unsafe {
        let fromp: *mut MaybeUninit<T> = mem::transmute(fromp.add(i));
        let top: *mut Out = mem::transmute(top.add(i));
        for j in 0..N {
          top
            .add(j)
            .write(f(fromp.add(j).replace(MaybeUninit::uninit()).assume_init()));
        }
      }
    }
    unsafe { to.assume_init() }
  }

  /// Returns the transpose of the matrix.
  pub fn transpose(self) -> Matrix<T, { M }, { N }> {
    let mut from = MaybeUninit::new(self);
    let mut trans = MaybeUninit::<[Vector<T, { M }>; N]>::uninit();
    let fromp: *mut Vector<MaybeUninit<T>, { N }> = unsafe { mem::transmute(&mut from) };
    let transp: *mut Vector<T, { M }> = unsafe { mem::transmute(&mut trans) };
    for j in 0..N {
      // Fetch the current row
      let mut row = MaybeUninit::<[T; M]>::uninit();
      let rowp: *mut T = unsafe { mem::transmute(&mut row) };
      for k in 0..M {
        unsafe {
          let fromp: *mut MaybeUninit<T> = mem::transmute(fromp.add(k));
          rowp
            .add(k)
            .write(fromp.add(j).replace(MaybeUninit::uninit()).assume_init());
        }
      }
      let row = Vector::<T, { M }>::from(unsafe { row.assume_init() });
      unsafe {
        transp.add(j).write(row);
      }
    }
    Matrix::<T, { M }, { N }>(unsafe { trans.assume_init() })
  }
}

impl<T, const N: usize> Matrix<T, { N }, { N }>
where
  T: Clone,
{
  /// Return the diagonal of the matrix. Only available for square matrices.
  pub fn diagonal(&self) -> Vector<T, { N }> {
    let mut diag = MaybeUninit::<[T; N]>::uninit();
    let diagp: *mut T = unsafe { mem::transmute(&mut diag) };
    for i in 0..N {
      unsafe {
        diagp.add(i).write(self.0[i].0[i].clone());
      }
    }
    Vector::<T, { N }>(unsafe { diag.assume_init() })
  }
}

impl<T> Matrix<T, 3, 3>
where
  T: Copy + PartialOrd + Product + Real + One + Zero,
  T: Neg<Output = T>,
  T: Add<T, Output = T> + Sub<T, Output = T>,
  T: Mul<T, Output = T> + Div<T, Output = T>,
  T: Zero,
  Self: Add<Self>,
  Self: Sub<Self>,
  Self: Mul<Self>,
  Self: Mul<Vector<T, 3>, Output = Vector<T, 3>>,
{
  pub fn det(&self) -> T {
    let t11 = self[2][2] * self[1][1] - self[1][2] * self[2][1];
    let t12 = self[1][2] * self[2][0] - self[2][2] * self[1][0];
    let t13 = self[2][1] * self[1][0] - self[1][1] * self[2][0];
    self[0][0] * t11 + self[0][1] * t12 + self[0][2] * t13
  }

  pub fn inverse(&self) -> Option<Self> {
    let det = self.det();
    if det.eq(&T::zero()) {
      return None;
    }

    let invdet = T::one() / det;

    Some(Matrix([
      Vector([
        (self[2][2] * self[1][1] - self[1][2] * self[2][1]) * invdet,
        (self[0][2] * self[2][1] - self[2][2] * self[0][1]) * invdet,
        (self[1][2] * self[0][1] - self[0][2] * self[1][1]) * invdet,
      ]),
      Vector([
        (self[1][2] * self[2][0] - self[2][2] * self[1][0]) * invdet,
        (self[2][2] * self[0][0] - self[0][2] * self[2][0]) * invdet,
        (self[0][2] * self[1][0] - self[1][2] * self[0][0]) * invdet,
      ]),
      Vector([
        (self[2][1] * self[1][0] - self[1][1] * self[2][0]) * invdet,
        (self[0][1] * self[2][0] - self[2][1] * self[0][0]) * invdet,
        (self[1][1] * self[0][0] - self[0][1] * self[1][0]) * invdet,
      ]),
    ]))
  }
}

impl<T, const N: usize> Matrix<T, { N }, { N }>
where
  T: Clone + PartialOrd + Product + Real + One + Zero,
  T: Neg<Output = T>,
  T: Add<T, Output = T> + Sub<T, Output = T>,
  T: Mul<T, Output = T> + Div<T, Output = T>,
  Self: Add<Self>,
  Self: Sub<Self>,
  Self: Mul<Self>,
  Self: Mul<Vector<T, { N }>, Output = Vector<T, { N }>>,
{
  /// Returns the [LU decomposition](https://en.wikipedia.org/wiki/LU_decomposition) of
  /// the matrix, if one exists.
  pub fn lu(mut self) -> Option<LU<T, { N }>> {
    let mut p = Permutation::<{ N }>::unit();

    for i in 0..N {
      let mut max_a = T::zero();
      let mut imax = i;
      for k in i..N {
        let abs = self[i][k].clone().abs();
        if abs > max_a {
          max_a = abs;
          imax = k;
        }
      }

      /* Check if matrix is degenerate */
      if max_a.is_zero() {
        return None;
      }

      /* Pivot rows */
      if imax != i {
        p.swap(i, imax);
        self.swap_rows(i, imax);
      }

      for j in i + 1..N {
        self[(j, i)] = self[(j, i)].clone() / self[(i, i)].clone();
        for k in i + 1..N {
          self[(j, k)] = self[(j, k)].clone() - self[(j, i)].clone() * self[(i, k)].clone();
        }
      }
    }
    Some(LU(p, self))
  }

  /// Returns the [determinant](https://en.wikipedia.org/wiki/Determinant) of
  /// the matrix.
  pub fn determinant(&self) -> T {
    self.clone().lu().map_or(T::zero(), |x| x.determinant())
  }

  /// Attempt to invert the matrix. For square matrices greater in size than
  /// three, [LU] decomposition is guaranteed to be used.
  pub fn invert(self) -> Option<Self> {
    self.lu().map(|x| x.invert())
  }
}

impl<T, const N: usize, const M: usize> From<[Vector<T, { N }>; M]> for Matrix<T, { N }, { M }> {
  fn from(array: [Vector<T, { N }>; M]) -> Self {
    Matrix::<T, { N }, { M }>(array)
  }
}

impl<T, const N: usize, const M: usize> From<[[T; N]; M]> for Matrix<T, { N }, { M }> {
  fn from(array: [[T; N]; M]) -> Self {
    let mut array = MaybeUninit::<[[T; N]; M]>::new(array);
    let mut vec_array: MaybeUninit<[Vector<T, { N }>; M]> = MaybeUninit::uninit();
    let arrayp: *mut MaybeUninit<[T; N]> = unsafe { mem::transmute(&mut array) };
    let vec_arrayp: *mut Vector<T, { N }> = unsafe { mem::transmute(&mut vec_array) };
    for i in 0..M {
      unsafe {
        vec_arrayp.add(i).write(Vector::<T, { N }>(
          arrayp.add(i).replace(MaybeUninit::uninit()).assume_init(),
        ));
      }
    }
    Matrix::<T, { N }, { M }>(unsafe { vec_array.assume_init() })
  }
}

impl<T> From<Quaternion<T>> for Matrix3<T>
where
  // This is really annoying to implement with
  T: Add + Mul + Sub + Real + One + Copy + Clone,
{
  fn from(quat: Quaternion<T>) -> Self {
    // Taken from cgmath
    let x2 = quat.v.x() + quat.v.x();
    let y2 = quat.v.y() + quat.v.y();
    let z2 = quat.v.z() + quat.v.z();

    let xx2 = x2 * quat.v.x();
    let xy2 = x2 * quat.v.y();
    let xz2 = x2 * quat.v.z();

    let yy2 = y2 * quat.v.y();
    let yz2 = y2 * quat.v.z();
    let zz2 = z2 * quat.v.z();

    let sy2 = y2 * quat.s;
    let sz2 = z2 * quat.s;
    let sx2 = x2 * quat.s;

    matrix![
      [T::one() - yy2 - zz2, xy2 + sz2, xz2 - sy2],
      [xy2 - sz2, T::one() - xx2 - zz2, yz2 + sx2],
      [xz2 + sy2, yz2 - sx2, T::one() - xx2 - yy2],
    ]
  }
}
