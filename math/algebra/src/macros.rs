#[macro_export]
macro_rules! impl_as_ref_self {
  ($ArrayN:ident) => {
    impl<T> AsRef<Self> for $ArrayN<T> {
      fn as_ref(&self) -> &Self {
        self
      }
    }

    impl<T> AsMut<Self> for $ArrayN<T> {
      fn as_mut(&mut self) -> &mut Self {
        self
      }
    }
  };
}

/// Generate array conversion implementations for a compound array type
#[macro_export]
macro_rules! impl_fixed_array_conversions {
  ($ArrayN:ident <$S:ident> { $($field:ident : $index:expr),+ }, $n:expr) => {
    impl<$S> Into<[$S; $n]> for $ArrayN<$S> {
      #[inline]
      fn into(self) -> [$S; $n] {
        match self { $ArrayN { $($field),+ } => [$($field),+] }
      }
    }

    impl<$S> AsRef<[$S; $n]> for $ArrayN<$S> {
      #[inline]
      fn as_ref(&self) -> &[$S; $n] {
        unsafe { std::mem::transmute(self) }
      }
    }

    impl<$S> AsMut<[$S; $n]> for $ArrayN<$S> {
      #[inline]
      fn as_mut(&mut self) -> &mut [$S; $n] {
        unsafe { std::mem::transmute(self) }
      }
    }

    impl<$S: Clone> From<[$S; $n]> for $ArrayN<$S> {
      #[inline]
      fn from(v: [$S; $n]) -> $ArrayN<$S> {
        // We need to use a clone here because we can't pattern match on arrays yet
        $ArrayN { $($field: v[$index].clone()),+ }
      }
    }

    impl<'a, $S> From<&'a [$S; $n]> for &'a $ArrayN<$S> {
      #[inline]
      fn from(v: &'a [$S; $n]) -> &'a $ArrayN<$S> {
        unsafe { std::mem::transmute(v) }
      }
    }

    impl<'a, $S> From<&'a mut [$S; $n]> for &'a mut $ArrayN<$S> {
      #[inline]
      fn from(v: &'a mut [$S; $n]) -> &'a mut $ArrayN<$S> {
        unsafe { std::mem::transmute(v) }
      }
    }
  }
}

/// Generate homogeneous tuple conversion implementations for a compound array type
#[macro_export]
macro_rules! impl_tuple_conversions {
  ($ArrayN:ident <$S:ident> { $($field:ident),+ }, $Tuple:ty) => {
    impl<$S> Into<$Tuple> for $ArrayN<$S> {
      #[inline]
      fn into(self) -> $Tuple {
        match self { $ArrayN { $($field),+ } => ($($field),+,) }
      }
    }

    impl<$S> AsRef<$Tuple> for $ArrayN<$S> {
      #[inline]
      fn as_ref(&self) -> &$Tuple {
        unsafe { std::mem::transmute(self) }
      }
    }

    impl<$S> AsMut<$Tuple> for $ArrayN<$S> {
      #[inline]
      fn as_mut(&mut self) -> &mut $Tuple {
        unsafe { std::mem::transmute(self) }
      }
    }

    impl<$S> From<$Tuple> for $ArrayN<$S> {
      #[inline]
      fn from(v: $Tuple) -> $ArrayN<$S> {
        match v { ($($field),+,) => $ArrayN { $($field),+ } }
      }
    }

    impl<'a, $S> From<&'a $Tuple> for &'a $ArrayN<$S> {
      #[inline]
      fn from(v: &'a $Tuple) -> &'a $ArrayN<$S> {
        unsafe { std::mem::transmute(v) }
      }
    }

    impl<'a, $S> From<&'a mut $Tuple> for &'a mut $ArrayN<$S> {
      #[inline]
      fn from(v: &'a mut $Tuple) -> &'a mut $ArrayN<$S> {
        unsafe { std::mem::transmute(v) }
      }
    }
  }
}

/// Generates index operators for a compound type
#[macro_export]
macro_rules! impl_index_operators {
  ($VectorN:ident<$S:ident>, $n:expr, $Output:ty, $I:ty) => {
    impl<$S> std::ops::Index<$I> for $VectorN<$S> {
      type Output = $Output;

      #[inline]
      fn index(&self, i: $I) -> &$Output {
        let v: &[$S; $n] = self.as_ref();
        &v[i]
      }
    }

    impl<$S> std::ops::IndexMut<$I> for $VectorN<$S> {
      #[inline]
      fn index_mut(&mut self, i: $I) -> &mut $Output {
        let v: &mut [$S; $n] = self.as_mut();
        &mut v[i]
      }
    }
  };
}

#[macro_export]
macro_rules! default_fn {
  { $($tt:tt)* } => { fn $( $tt )* };
}

/// Generates a binary operator implementation for the permutations of by-ref and by-val
#[macro_export]
macro_rules! impl_operator {
  // When it is an unary operator
  (<$S:ident> $Op:ident for $Lhs:ty {
    fn $op:ident($x:ident) -> $Output:ty { $body:expr }
  }) => {
    impl<$S: $Op<Output = $S> + Copy> $Op for $Lhs {
      type Output = $Output;
      default_fn!($op(self) -> $Output {
        let $x = self; $body
      });
    }
  };
  // When the right operand is a scalar
  (<$S:ident> $Op:ident<$Rhs:ident> for $Lhs:ty {
    fn $op:ident($lhs:ident, $rhs:ident) -> $Output:ty { $body:expr }
  }) => {
    impl<$S: $Op<Output = $S> + Copy> $Op<$Rhs> for $Lhs {
      type Output = $Output;
      default_fn!($op(self, other: $Rhs) -> $Output {
        let ($lhs, $rhs) = (self, other); $body
      });
    }
  };
  // When the right operand is a compound type
  (<$S:ident> $Op:ident<$Rhs:ty> for $Lhs:ty {
    fn $op:ident($lhs:ident, $rhs:ident) -> $Output:ty { $body:expr }
  }) => {
    impl<$S: $Op<Output = $S> + Copy> $Op<$Rhs> for $Lhs {
      type Output = $Output;
      default_fn!( $op(self, other: $Rhs) -> $Output {
        let ($lhs, $rhs) = (self, other); $body
      });
    }
  };
  // When the left operand is a scalar
  ($Op:ident<$Rhs:ident<$S:ident>> for $Lhs:ty {
    fn $op:ident($lhs:ident, $rhs:ident) -> $Output:ty { $body:expr }
  }) => {
    impl $Op<$Rhs<$S>> for $Lhs {
      type Output = $Output;
      default_fn!( $op(self, other: $Rhs<$S>) -> $Output {
        let ($lhs, $rhs) = (self, other); $body
      });
    }
  };
}

#[macro_export]
macro_rules! impl_assignment_operator {
  (<$S:ident> $Op:ident<$Rhs:ty> for $Lhs:ty {
      fn $op:ident(&mut $lhs:ident, $rhs:ident) $body:block
  }) => {
      impl<$S: $Op<$S> + Copy> $Op<$Rhs> for $Lhs {
          default_fn!( $op(&mut $lhs, $rhs: $Rhs) $body );
      }
  };
}

#[macro_export]
macro_rules! impl_as_ptr {
  ($Item:ident) => {
    impl<T> $Item<T> {
      pub fn as_ptr(&self) -> *const Self {
        self
      }
    }
  };
}
