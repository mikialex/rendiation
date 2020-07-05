pub mod vec2;
pub mod vec3;
pub mod vec4;
pub use vec2::*;
pub use vec3::*;
pub use vec4::*;

use std::fmt::Debug;
use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Neg, Not, Rem, Sub};
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};
use std::{f32, f64};

use super::consts::*;

use crate::*;

macro_rules! impl_vector {
  ($VectorN:ident { $($field:ident),+ }, $n:expr, $constructor:ident) => {
    impl<S> $VectorN<S> {
      /// Construct a new vector, using the provided values.
      #[inline]
      pub const fn new($($field: S),+) -> $VectorN<S> {
        $VectorN { $($field: $field),+ }
      }

      /// return the length of element
      #[inline]
      pub fn len() -> usize {
      return $n;
      }

      /// Perform the given operation on each field in the vector, returning a new point
      /// constructed from the operations.
      #[inline]
      pub fn map<U, F>(self, mut f: F) -> $VectorN<U>
        where F: FnMut(S) -> U
      {
        $VectorN { $($field: f(self.$field)),+ }
      }

      /// Construct a new vector where each component is the result of
      /// applying the given operation to each pair of components of the
      /// given vectors.
      #[inline]
      pub fn zip<S2, S3, F>(self, v2: $VectorN<S2>, mut f: F) -> $VectorN<S3>
        where F: FnMut(S, S2) -> S3
      {
        $VectorN { $($field: f(self.$field, v2.$field)),+ }
      }
    }

    /// The short constructor.
    #[inline]
    pub const fn $constructor<S>($($field: S),+) -> $VectorN<S> {
      $VectorN::new($($field),+)
    }

    impl_index_operators!($VectorN<S>, $n, S, usize);
    impl_index_operators!($VectorN<S>, $n, [S], std::ops::Range<usize>);
    impl_index_operators!($VectorN<S>, $n, [S], std::ops::RangeTo<usize>);
    impl_index_operators!($VectorN<S>, $n, [S], std::ops::RangeFrom<usize>);
    impl_index_operators!($VectorN<S>, $n, [S], std::ops::RangeFull);

    impl_scalar_ops!($VectorN<usize> { $($field),+ });
  }
}

macro_rules! impl_scalar_ops {
  ($VectorN:ident<$S:ident> { $($field:ident),+ }) => {
    impl_operator!(Mul<$VectorN<$S>> for $S {
      fn mul(scalar, vector) -> $VectorN<$S> { $VectorN::new($(scalar * vector.$field),+) }
    });
    impl_operator!(Div<$VectorN<$S>> for $S {
      fn div(scalar, vector) -> $VectorN<$S> { $VectorN::new($(scalar / vector.$field),+) }
    });
    impl_operator!(Rem<$VectorN<$S>> for $S {
      fn rem(scalar, vector) -> $VectorN<$S> { $VectorN::new($(scalar % vector.$field),+) }
    });
  };
}

impl_vector!(Vec2 { x, y }, 2, vec2);
impl_vector!(Vec3 { x, y, z }, 3, vec3);
impl_vector!(Vec4 { x, y, z, w }, 4, vec4);

impl_fixed_array_conversions!(Vec2<T> { x: 0, y: 1 }, 2);
impl_fixed_array_conversions!(Vec3<T> { x: 0, y: 1, z: 2 }, 3);
impl_fixed_array_conversions!(Vec4<T> { x: 0, y: 1, z: 2, w: 3 }, 4);

impl_tuple_conversions!(Vec2<T> { x, y }, (T, T));
impl_tuple_conversions!(Vec3<T> { x, y, z }, (T, T, T));
impl_tuple_conversions!(Vec4<T> { x, y, z, w }, (T, T, T, T));

pub trait Arithmetic:
  Debug
  + Copy
  + Clone
  + Add<Self, Output = Self>
  + Sub<Self, Output = Self>
  + Mul<Self, Output = Self>
  + Div<Self, Output = Self>
  + Rem<Self, Output = Self>
  + AddAssign<Self>
  + SubAssign<Self>
  + MulAssign<Self>
  + DivAssign<Self>
  + Neg<Output = Self>
  + Cmp
  + One
  + Two
  + Zero
  + Half
{
}

impl Arithmetic for f32 {}
impl Arithmetic for f64 {}

pub trait Cmp {
  type Bool: Copy
    + Not<Output = Self::Bool>
    + BitAnd<Self::Bool, Output = Self::Bool>
    + BitOr<Self::Bool, Output = Self::Bool>
    + BitXor<Self::Bool, Output = Self::Bool>;

  fn eq(self, rhs: Self) -> bool;
  fn ne(self, rhs: Self) -> bool;
  fn gt(self, rhs: Self) -> bool;
  fn lt(self, rhs: Self) -> bool;
  fn ge(self, rhs: Self) -> bool;
  fn le(self, rhs: Self) -> bool;
}

impl Cmp for f32 {
  type Bool = bool;

  #[inline(always)]
  fn eq(self, rhs: Self) -> bool {
    self == rhs
  }
  #[inline(always)]
  fn ne(self, rhs: Self) -> bool {
    self != rhs
  }
  #[inline(always)]
  fn gt(self, rhs: Self) -> bool {
    self > rhs
  }
  #[inline(always)]
  fn lt(self, rhs: Self) -> bool {
    self < rhs
  }
  #[inline(always)]
  fn ge(self, rhs: Self) -> bool {
    self >= rhs
  }
  #[inline(always)]
  fn le(self, rhs: Self) -> bool {
    self <= rhs
  }
}

impl Cmp for f64 {
  type Bool = bool;

  #[inline(always)]
  fn eq(self, rhs: Self) -> bool {
    self == rhs
  }
  #[inline(always)]
  fn ne(self, rhs: Self) -> bool {
    self != rhs
  }
  #[inline(always)]
  fn gt(self, rhs: Self) -> bool {
    self > rhs
  }
  #[inline(always)]
  fn lt(self, rhs: Self) -> bool {
    self < rhs
  }
  #[inline(always)]
  fn ge(self, rhs: Self) -> bool {
    self >= rhs
  }
  #[inline(always)]
  fn le(self, rhs: Self) -> bool {
    self <= rhs
  }
}
