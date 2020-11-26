use crate::*;

impl<T, const N: usize, const M: usize> Clone for Matrix<T, { N }, { M }>
where
  T: Clone,
{
  fn clone(&self) -> Self {
    Matrix::<T, { N }, { M }>(self.0.clone())
  }
}

impl<T, const N: usize, const M: usize> Copy for Matrix<T, { N }, { M }> where T: Copy {}

impl<T, const N: usize, const M: usize> Deref for Matrix<T, { N }, { M }> {
  type Target = [Vector<T, { N }>; M];

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T, const N: usize, const M: usize> DerefMut for Matrix<T, { N }, { M }> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl<T, const N: usize, const M: usize> Hash for Matrix<T, { N }, { M }>
where
  T: Hash,
{
  fn hash<H: Hasher>(&self, state: &mut H) {
    for i in 0..M {
      self.0[i].hash(state);
    }
  }
}

impl<T, const N: usize, const M: usize> FromIterator<T> for Matrix<T, { N }, { M }> {
  fn from_iter<I>(iter: I) -> Self
  where
    I: IntoIterator<Item = T>,
  {
    let mut iter = iter.into_iter();
    let mut new = MaybeUninit::<[Vector<T, { N }>; M]>::uninit();
    let newp: *mut Vector<T, { N }> = unsafe { mem::transmute(&mut new) };

    for i in 0..M {
      let mut newv = MaybeUninit::<Vector<T, { N }>>::uninit();
      let newvp: *mut T = unsafe { mem::transmute(&mut newv) };
      for j in 0..N {
        if let Some(next) = iter.next() {
          unsafe { newvp.add(j).write(next) };
        } else {
          panic!(
            "too few items in iterator to create Matrix<_, {}, {}>",
            N, M
          );
        }
      }
      unsafe {
        newp
          .add(i)
          .write(mem::replace(&mut newv, MaybeUninit::uninit()).assume_init());
      }
    }

    if iter.next().is_some() {
      panic!(
        "too many items in iterator to create Matrix<_, {}, {}>",
        N, M
      );
    }

    Matrix::<T, { N }, { M }>(unsafe { new.assume_init() })
  }
}

impl<T, const N: usize, const M: usize> FromIterator<Vector<T, { N }>> for Matrix<T, { N }, { M }> {
  fn from_iter<I>(iter: I) -> Self
  where
    I: IntoIterator<Item = Vector<T, { N }>>,
  {
    let mut iter = iter.into_iter();
    let mut new = MaybeUninit::<[Vector<T, { N }>; M]>::uninit();
    let newp: *mut Vector<T, { N }> = unsafe { mem::transmute(&mut new) };

    for i in 0..M {
      if let Some(v) = iter.next() {
        unsafe {
          newp.add(i).write(v);
        }
      } else {
        panic!(
          "too few items in iterator to create Matrix<_, {}, {}>",
          N, M
        );
      }
    }
    Matrix::<T, { N }, { M }>(unsafe { new.assume_init() })
  }
}

impl<T, const N: usize, const M: usize> IntoIterator for Matrix<T, { N }, { M }> {
  type Item = Vector<T, { N }>;
  type IntoIter = ArrayIter<Vector<T, { N }>, { M }>;

  fn into_iter(self) -> Self::IntoIter {
    let Matrix(array) = self;
    ArrayIter {
      array: MaybeUninit::new(array),
      pos: 0,
    }
  }
}

impl<T, const N: usize, const M: usize> Index<usize> for Matrix<T, { N }, { M }> {
  type Output = Vector<T, { N }>;

  fn index(&self, column: usize) -> &Self::Output {
    &self.0[column]
  }
}

impl<T, const N: usize, const M: usize> IndexMut<usize> for Matrix<T, { N }, { M }> {
  fn index_mut(&mut self, column: usize) -> &mut Self::Output {
    &mut self.0[column]
  }
}

impl<T, const N: usize, const M: usize> Index<(usize, usize)> for Matrix<T, { N }, { M }> {
  type Output = T;

  fn index(&self, (row, column): (usize, usize)) -> &Self::Output {
    &self.0[column][row]
  }
}

impl<T, const N: usize, const M: usize> IndexMut<(usize, usize)> for Matrix<T, { N }, { M }> {
  fn index_mut(&mut self, (row, column): (usize, usize)) -> &mut Self::Output {
    &mut self.0[column][row]
  }
}

impl<A, B, RHS, const N: usize, const M: usize> PartialEq<RHS> for Matrix<A, { N }, { M }>
where
  RHS: Deref<Target = [Vector<B, { N }>; M]>,
  A: PartialEq<B>,
{
  fn eq(&self, other: &RHS) -> bool {
    for (a, b) in self.0.iter().zip(other.deref().iter()) {
      if !a.eq(b) {
        return false;
      }
    }
    true
  }
}

/// I'm not quite sure how to format the debug output for a matrix.
impl<T, const N: usize, const M: usize> fmt::Debug for Matrix<T, { N }, { M }>
where
  T: fmt::Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Matrix [ ")?;
    for i in 0..N {
      write!(f, "[ ")?;
      for j in 0..M {
        write!(f, "{:?} ", self.0[j].0[i])?;
      }
      write!(f, "] ")?;
    }
    write!(f, "]")
  }
}
