pub use crate::*;

/// Permutation matrix created for LU decomposition.
#[derive(Copy, Clone)]
pub struct Permutation<const N: usize> {
  arr: [usize; N],
  num_swaps: usize,
}

impl<const N: usize> fmt::Debug for Permutation<{ N }> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "[ ")?;
    for i in 0..N {
      write!(f, "{:?} ", self.arr[i])?;
    }
    write!(f, "] ")
  }
}

impl<RHS, const N: usize> PartialEq<RHS> for Permutation<{ N }>
where
  RHS: Deref<Target = [usize; N]>,
{
  fn eq(&self, other: &RHS) -> bool {
    for (a, b) in self.arr.iter().zip(other.deref().iter()) {
      if !a.eq(b) {
        return false;
      }
    }
    true
  }
}

impl<const N: usize> Deref for Permutation<{ N }> {
  type Target = [usize; N];

  fn deref(&self) -> &Self::Target {
    &self.arr
  }
}

impl<const N: usize> DerefMut for Permutation<{ N }> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.arr
  }
}

impl<const N: usize> Permutation<{ N }> {
  /// Returns the unit permutation.
  pub fn unit() -> Permutation<{ N }> {
    let mut arr: [MaybeUninit<usize>; N] = MaybeUninit::uninit_array();
    let arr = unsafe {
      for i in 0..N {
        arr[i] = MaybeUninit::new(i);
      }
      transmute_copy::<_, _>(&arr)
    };
    Permutation { arr, num_swaps: 0 }
  }

  /// Swaps two rows and increments the number of swaps.
  pub fn swap(&mut self, a: usize, b: usize) {
    self.num_swaps += 1;
    self.arr.swap(a, b);
  }

  /// Returns the number of swaps that have occurred.
  pub fn num_swaps(&self) -> usize {
    self.num_swaps
  }
}

impl<T, const N: usize> Mul<Vector<T, { N }>> for Permutation<{ N }>
where
  // The clone bound can be
  // removed from here at some
  // point with better written
  // code.
  T: Clone,
{
  type Output = Vector<T, { N }>;

  fn mul(self, rhs: Vector<T, { N }>) -> Self::Output {
    Vector::from_iter((0..N).map(|i| rhs[self[i]].clone()))
  }
}

/// The result of LU factorizing a square matrix with partial-pivoting.
#[derive(Copy, Clone, Debug)]
pub struct LU<T, const N: usize>(pub Permutation<{ N }>, pub Matrix<T, { N }, { N }>);

impl<T, const N: usize> Index<(usize, usize)> for LU<T, { N }> {
  type Output = T;

  fn index(&self, (row, column): (usize, usize)) -> &Self::Output {
    &self.1[(row, column)]
  }
}

impl<T, const N: usize> LU<T, { N }>
where
  T: Clone
    + PartialEq
    + One
    + Zero
    + Product
    + Neg<Output = T>
    + Sub<T, Output = T>
    + Mul<T, Output = T>
    + Div<T, Output = T>,
{
  /// Returns the permutation sequence of the factorization.
  pub fn p(&self) -> &Permutation<{ N }> {
    &self.0
  }

  /// Solves the linear equation `self * x = b` and returns `x`.
  pub fn solve(&self, b: Vector<T, { N }>) -> Vector<T, { N }> {
    let mut x = self.0.clone() * b;
    for i in 0..N {
      for k in 0..i {
        x[i] = x[i].clone() - self[(i, k)].clone() * x[k].clone();
      }
    }

    for i in (0..N).rev() {
      for k in i + 1..N {
        x[i] = x[i].clone() - self[(i, k)].clone() * x[k].clone();
      }

      // TODO(map): Consider making DivAssign a requirement so that we
      // don't have to clone here.
      x[i] = x[i].clone() / self[(i, i)].clone();
    }
    x
  }

  /// Returns the determinant of the matrix.
  pub fn determinant(&self) -> T {
    let det: T = self.1.diagonal().into_iter().product();
    if self.0.num_swaps % 2 == 1 {
      -det
    } else {
      det
    }
  }

  /// Returns the inverse of the matrix, which is certain to exist.
  pub fn invert(self) -> Matrix<T, { N }, { N }> {
    Matrix::<T, { N }, { N }>::one()
      .into_iter()
      .map(|col| self.solve(col))
      .collect()
  }
}
