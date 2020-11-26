use crate::*;

/// Element-wise addition of two equal sized matrices.
impl<A, B, const N: usize, const M: usize> Add<Matrix<B, { N }, { M }>> for Matrix<A, { N }, { M }>
where
  A: Add<B>,
{
  type Output = Matrix<<A as Add<B>>::Output, { N }, { M }>;

  fn add(self, rhs: Matrix<B, { N }, { M }>) -> Self::Output {
    let mut mat = MaybeUninit::<[Vector<<A as Add<B>>::Output, { N }>; M]>::uninit();
    let mut lhs = MaybeUninit::new(self);
    let mut rhs = MaybeUninit::new(rhs);
    let matp: *mut Vector<<A as Add<B>>::Output, { N }> = unsafe { mem::transmute(&mut mat) };
    let lhsp: *mut MaybeUninit<Vector<A, { N }>> = unsafe { mem::transmute(&mut lhs) };
    let rhsp: *mut MaybeUninit<Vector<B, { N }>> = unsafe { mem::transmute(&mut rhs) };
    for i in 0..M {
      unsafe {
        matp.add(i).write(
          lhsp.add(i).replace(MaybeUninit::uninit()).assume_init()
            + rhsp.add(i).replace(MaybeUninit::uninit()).assume_init(),
        );
      }
    }
    Matrix::<<A as Add<B>>::Output, { N }, { M }>(unsafe { mat.assume_init() })
  }
}

impl<A, B, const N: usize, const M: usize> AddAssign<Matrix<B, { N }, { M }>>
  for Matrix<A, { N }, { M }>
where
  A: AddAssign<B>,
{
  fn add_assign(&mut self, rhs: Matrix<B, { N }, { M }>) {
    let mut rhs = MaybeUninit::new(rhs);
    let rhsp: *mut MaybeUninit<Vector<B, { N }>> = unsafe { mem::transmute(&mut rhs) };
    for i in 0..M {
      self.0[i] += unsafe { rhsp.add(i).replace(MaybeUninit::uninit()).assume_init() };
    }
  }
}

/// Element-wise subtraction of two equal sized matrices.
impl<A, B, const N: usize, const M: usize> Sub<Matrix<B, { N }, { M }>> for Matrix<A, { N }, { M }>
where
  A: Sub<B>,
{
  type Output = Matrix<<A as Sub<B>>::Output, { N }, { M }>;

  fn sub(self, rhs: Matrix<B, { N }, { M }>) -> Self::Output {
    let mut mat = MaybeUninit::<[Vector<<A as Sub<B>>::Output, { N }>; M]>::uninit();
    let mut lhs = MaybeUninit::new(self);
    let mut rhs = MaybeUninit::new(rhs);
    let matp: *mut Vector<<A as Sub<B>>::Output, { N }> = unsafe { mem::transmute(&mut mat) };
    let lhsp: *mut MaybeUninit<Vector<A, { N }>> = unsafe { mem::transmute(&mut lhs) };
    let rhsp: *mut MaybeUninit<Vector<B, { N }>> = unsafe { mem::transmute(&mut rhs) };
    for i in 0..M {
      unsafe {
        matp.add(i).write(
          lhsp.add(i).replace(MaybeUninit::uninit()).assume_init()
            - rhsp.add(i).replace(MaybeUninit::uninit()).assume_init(),
        );
      }
    }
    Matrix::<<A as Sub<B>>::Output, { N }, { M }>(unsafe { mat.assume_init() })
  }
}

impl<A, B, const N: usize, const M: usize> SubAssign<Matrix<B, { N }, { M }>>
  for Matrix<A, { N }, { M }>
where
  A: SubAssign<B>,
{
  fn sub_assign(&mut self, rhs: Matrix<B, { N }, { M }>) {
    let mut rhs = MaybeUninit::new(rhs);
    let rhsp: *mut MaybeUninit<Vector<B, { N }>> = unsafe { mem::transmute(&mut rhs) };
    for i in 0..M {
      self.0[i] -= unsafe { rhsp.add(i).replace(MaybeUninit::uninit()).assume_init() };
    }
  }
}

impl<T, const N: usize, const M: usize> Neg for Matrix<T, { N }, { M }>
where
  T: Neg,
{
  type Output = Matrix<<T as Neg>::Output, { N }, { M }>;

  fn neg(self) -> Self::Output {
    let mut from = MaybeUninit::new(self);
    let mut mat = MaybeUninit::<[Vector<<T as Neg>::Output, { N }>; M]>::uninit();
    let fromp: *mut MaybeUninit<Vector<T, { N }>> = unsafe { mem::transmute(&mut from) };
    let matp: *mut Vector<<T as Neg>::Output, { N }> = unsafe { mem::transmute(&mut mat) };
    for i in 0..M {
      unsafe {
        matp.add(i).write(
          fromp
            .add(i)
            .replace(MaybeUninit::uninit())
            .assume_init()
            .neg(),
        );
      }
    }
    Matrix::<<T as Neg>::Output, { N }, { M }>(unsafe { mat.assume_init() })
  }
}

impl<T, const N: usize, const M: usize, const P: usize> Mul<Matrix<T, { M }, { P }>>
  for Matrix<T, { N }, { M }>
where
  T: Add<T, Output = T> + Mul<T, Output = T> + Clone,
  Vector<T, { M }>: InnerSpace,
{
  type Output = Matrix<<Vector<T, { M }> as VectorSpace>::Scalar, { N }, { P }>;

  fn mul(self, rhs: Matrix<T, { M }, { P }>) -> Self::Output {
    // It might not seem that Rust's type system is helping me at all here,
    // but that's absolutely not true. I got the arrays iterations wrong on
    // the first try and Rust was nice enough to inform me of that fact.
    let mut mat =
      MaybeUninit::<[Vector<<Vector<T, { M }> as VectorSpace>::Scalar, { N }>; P]>::uninit();
    let matp: *mut Vector<<Vector<T, { M }> as VectorSpace>::Scalar, { N }> =
      unsafe { mem::transmute(&mut mat) };
    for i in 0..P {
      let mut column = MaybeUninit::<[<Vector<T, { M }> as VectorSpace>::Scalar; N]>::uninit();
      let columnp: *mut <Vector<T, { M }> as VectorSpace>::Scalar =
        unsafe { mem::transmute(&mut column) };
      for j in 0..N {
        // Fetch the current row:
        let mut row = MaybeUninit::<[T; M]>::uninit();
        let rowp: *mut T = unsafe { mem::transmute(&mut row) };
        for k in 0..M {
          unsafe {
            rowp.add(k).write(self.0[k].0[j].clone());
          }
        }
        let row = Vector::<T, { M }>::from(unsafe { row.assume_init() });
        unsafe {
          columnp.add(j).write(row.dot(rhs.0[i].clone()));
        }
      }
      let column =
        Vector::<<Vector<T, { M }> as VectorSpace>::Scalar, { N }>(unsafe { column.assume_init() });
      unsafe {
        matp.add(i).write(column);
      }
    }
    Matrix::<<Vector<T, { M }> as VectorSpace>::Scalar, { N }, { P }>(unsafe { mat.assume_init() })
  }
}

impl<T, const N: usize, const M: usize> Mul<Vector<T, { M }>> for Matrix<T, { N }, { M }>
where
  T: Add<T, Output = T> + Mul<T, Output = T> + Clone,
  Vector<T, { M }>: InnerSpace,
{
  type Output = Vector<<Vector<T, { M }> as VectorSpace>::Scalar, { N }>;

  fn mul(self, rhs: Vector<T, { M }>) -> Self::Output {
    let mut column = MaybeUninit::<[<Vector<T, { M }> as VectorSpace>::Scalar; N]>::uninit();
    let columnp: *mut <Vector<T, { M }> as VectorSpace>::Scalar =
      unsafe { mem::transmute(&mut column) };
    for j in 0..N {
      // Fetch the current row:
      let mut row = MaybeUninit::<[T; M]>::uninit();
      let rowp: *mut T = unsafe { mem::transmute(&mut row) };
      for k in 0..M {
        unsafe {
          rowp.add(k).write(self.0[k].0[j].clone());
        }
      }
      let row = Vector::<T, { M }>::from(unsafe { row.assume_init() });
      unsafe {
        columnp.add(j).write(row.dot(rhs.clone()));
      }
    }
    Vector::<<Vector<T, { M }> as VectorSpace>::Scalar, { N }>(unsafe { column.assume_init() })
  }
}

/// Scalar multiply
impl<T, const N: usize, const M: usize> Mul<T> for Matrix<T, { N }, { M }>
where
  T: Mul<T, Output = T> + Clone,
{
  type Output = Matrix<T, { N }, { M }>;

  fn mul(self, scalar: T) -> Self::Output {
    let mut mat = MaybeUninit::<[Vector<T, { N }>; M]>::uninit();
    let matp: *mut Vector<T, { N }> = unsafe { mem::transmute(&mut mat) };
    for i in 0..M {
      unsafe {
        matp.add(i).write(self.0[i].clone() * scalar.clone());
      }
    }
    Matrix::<T, { N }, { M }>(unsafe { mat.assume_init() })
  }
}

impl<const N: usize, const M: usize> Mul<Matrix<f32, { N }, { M }>> for f32 {
  type Output = Matrix<f32, { N }, { M }>;

  fn mul(self, mat: Matrix<f32, { N }, { M }>) -> Self::Output {
    mat.map(|x| x * self)
  }
}

impl<const N: usize, const M: usize> Mul<Matrix<f64, { N }, { M }>> for f64 {
  type Output = Matrix<f64, { N }, { M }>;

  fn mul(self, mat: Matrix<f64, { N }, { M }>) -> Self::Output {
    mat.map(|x| x * self)
  }
}
