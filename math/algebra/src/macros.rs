//! Internal code generation macros shared by the vector, matrix and quaternion types.
//!
//! They are exported because downstream crates occasionally use them to implement the same
//! conversions for their own `#[repr(C)]` homogeneous structs.

/// Implement `AsRef<Self>` and `AsMut<Self>`, so generic code that accepts `impl AsRef<X>` can
/// take `X` directly.
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

/// Generate the fixed size array conversions for a compound type.
///
/// The type must be `#[repr(C)]` and only contain `$n` fields of type `$S`, because the
/// reference conversions reinterpret the struct as `[$S; $n]` in place.
#[macro_export]
macro_rules! impl_fixed_array_conversions {
  ($ArrayN:ident <$S:ident> { $($field:ident : $index:expr),+ }, $n:expr) => {
    impl<$S> From<$ArrayN<$S>> for [$S; $n] {
      #[inline]
      fn from(v: $ArrayN<$S>) -> [$S; $n] {
        let $ArrayN { $($field),+ } = v;
        [$($field),+]
      }
    }

    impl<$S> AsRef<[$S; $n]> for $ArrayN<$S> {
      #[inline]
      fn as_ref(&self) -> &[$S; $n] {
        // safety: repr(C) struct of n fields of S has the same layout as [S; n]
        unsafe { &*(self as *const Self as *const [$S; $n]) }
      }
    }

    impl<$S> AsMut<[$S; $n]> for $ArrayN<$S> {
      #[inline]
      fn as_mut(&mut self) -> &mut [$S; $n] {
        // safety: see as_ref
        unsafe { &mut *(self as *mut Self as *mut [$S; $n]) }
      }
    }

    impl<$S> From<[$S; $n]> for $ArrayN<$S> {
      #[inline]
      fn from(v: [$S; $n]) -> $ArrayN<$S> {
        let [$($field),+] = v;
        $ArrayN { $($field),+ }
      }
    }

    impl<'a, $S> From<&'a [$S; $n]> for &'a $ArrayN<$S> {
      #[inline]
      fn from(v: &'a [$S; $n]) -> &'a $ArrayN<$S> {
        // safety: see as_ref
        unsafe { &*(v as *const [$S; $n] as *const $ArrayN<$S>) }
      }
    }

    impl<'a, $S> From<&'a mut [$S; $n]> for &'a mut $ArrayN<$S> {
      #[inline]
      fn from(v: &'a mut [$S; $n]) -> &'a mut $ArrayN<$S> {
        // safety: see as_ref
        unsafe { &mut *(v as *mut [$S; $n] as *mut $ArrayN<$S>) }
      }
    }
  }
}

/// Generate the by value homogeneous tuple conversions for a compound type.
///
/// Only by value conversions are provided, the memory layout of rust tuple is unspecified so
/// reinterpreting a reference is not sound.
#[macro_export]
macro_rules! impl_tuple_conversions {
  ($ArrayN:ident <$S:ident> { $($field:ident),+ }, $Tuple:ty) => {
    impl<$S> From<$ArrayN<$S>> for $Tuple {
      #[inline]
      fn from(v: $ArrayN<$S>) -> $Tuple {
        let $ArrayN { $($field),+ } = v;
        ($($field),+,)
      }
    }

    impl<$S> From<$Tuple> for $ArrayN<$S> {
      #[inline]
      fn from(v: $Tuple) -> $ArrayN<$S> {
        let ($($field),+,) = v;
        $ArrayN { $($field),+ }
      }
    }
  }
}

/// Generate the index operators for a compound type by viewing it as a fixed size array.
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

/// Generate a unary or binary operator implementation.
///
/// The supported forms are: unary operator, compound type with scalar rhs, compound type with
/// compound rhs, and scalar lhs with compound rhs (the latter must be implemented for each
/// concrete scalar type because of the orphan rule).
#[macro_export]
macro_rules! impl_operator {
  // When it is a unary operator
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

/// Generate a compound assignment operator implementation.
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

/// Generate `as_ptr`, mainly for passing data to graphics api.
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
