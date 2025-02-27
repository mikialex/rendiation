mod dimension;
mod mat2;
mod mat3;
mod mat4;
mod mat4x3;

use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::ops::{AddAssign, DivAssign, MulAssign, RemAssign, SubAssign};

pub use dimension::*;
pub use mat2::*;
pub use mat3::*;
pub use mat4::*;
pub use mat4x3::*;

use crate::*;

#[rustfmt::skip]
#[macro_export] 
macro_rules! matrix { 
  ($m11:expr, $m12:expr, 
   $m21:expr, $m22:expr) => {
    rendiation_algebra::Mat2::new(
      $m11, $m12,
      $m21, $m22,
    )
  };
  ($m11:expr, $m12:expr, $m13:expr, 
   $m21:expr, $m22:expr, $m23:expr, 
   $m31:expr, $m32:expr, $m33:expr) => {
    rendiation_algebra::Mat3::new(
      $m11, $m12, $m13,
      $m21, $m22, $m23,
      $m31, $m32, $m33
    )
  };
  ($m11:expr, $m12:expr, $m13:expr, $m14:expr, 
   $m21:expr, $m22:expr, $m23:expr, $m24:expr, 
   $m31:expr, $m32:expr, $m33:expr, $m34:expr,
   $m41:expr, $m42:expr, $m43:expr, $m44:expr) => {
    rendiation_algebra::Mat4::new(
      $m11, $m12, $m13, $m14,
      $m21, $m22, $m23, $m24,
      $m31, $m32, $m33, $m34,
      $m41, $m42, $m43, $m44,
    )
  };
}

macro_rules! impl_matrix {
  ($MatrixN:ident { $($field:ident),+ }, $n:expr, $constructor:ident) => {
    impl_operator!(<S> Neg for $MatrixN<S> {
      fn neg(matrix) -> $MatrixN<S> { $MatrixN { $($field: -matrix.$field),+ } }
    });

    impl_operator!(<S> Mul<S> for $MatrixN<S> {
      fn mul(matrix, scalar) -> $MatrixN<S> { $MatrixN { $($field: matrix.$field * scalar),+ } }
    });
    impl_operator!(<S> Div<S> for $MatrixN<S> {
      fn div(matrix, scalar) -> $MatrixN<S> { $MatrixN { $($field: matrix.$field / scalar),+ } }
    });
    impl_operator!(<S> Rem<S> for $MatrixN<S> {
      fn rem(matrix, scalar) -> $MatrixN<S> { $MatrixN { $($field: matrix.$field % scalar),+ } }
    });
    impl_assignment_operator!(<S> MulAssign<S> for $MatrixN<S> {
      fn mul_assign(&mut self, scalar) { $(self.$field *= scalar);+ }
    });
    impl_assignment_operator!(<S> DivAssign<S> for $MatrixN<S> {
      fn div_assign(&mut self, scalar) { $(self.$field /= scalar);+ }
    });
    impl_assignment_operator!(<S> RemAssign<S> for $MatrixN<S> {
      fn rem_assign(&mut self, scalar) { $(self.$field %= scalar);+ }
    });

    impl_operator!(<S> Add<$MatrixN<S> > for $MatrixN<S> {
      fn add(lhs, rhs) -> $MatrixN<S> { $MatrixN { $($field: lhs.$field + rhs.$field),+ } }
    });
    impl_operator!(<S> Sub<$MatrixN<S> > for $MatrixN<S> {
      fn sub(lhs, rhs) -> $MatrixN<S> { $MatrixN { $($field: lhs.$field - rhs.$field),+ } }
    });
    impl<S: AddAssign<S>> AddAssign<$MatrixN<S>> for $MatrixN<S> {
      fn add_assign(&mut self, other: $MatrixN<S>) { $(self.$field += other.$field);+ }
    }
    impl<S: SubAssign<S>> SubAssign<$MatrixN<S>> for $MatrixN<S> {
      fn sub_assign(&mut self, other: $MatrixN<S>) { $(self.$field -= other.$field);+ }
    }

    impl_scalar_ops!($MatrixN<usize> { $($field),+ });
    impl_scalar_ops!($MatrixN<u8>    { $($field),+ });
    impl_scalar_ops!($MatrixN<u16>   { $($field),+ });
    impl_scalar_ops!($MatrixN<u32>   { $($field),+ });
    impl_scalar_ops!($MatrixN<u64>   { $($field),+ });
    impl_scalar_ops!($MatrixN<isize> { $($field),+ });
    impl_scalar_ops!($MatrixN<i8>    { $($field),+ });
    impl_scalar_ops!($MatrixN<i16>   { $($field),+ });
    impl_scalar_ops!($MatrixN<i32>   { $($field),+ });
    impl_scalar_ops!($MatrixN<i64>   { $($field),+ });
    impl_scalar_ops!($MatrixN<f32>   { $($field),+ });
    impl_scalar_ops!($MatrixN<f64>   { $($field),+ });

  }
}

macro_rules! impl_scalar_ops {
  ($MatrixN:ident<$S:ident> { $($field:ident),+ }) => {
    impl_operator!(Mul<$MatrixN<$S>> for $S {
      fn mul(scalar, matrix) -> $MatrixN<$S> { $MatrixN { $($field: scalar * matrix.$field),+ } }
    });
    impl_operator!(Div<$MatrixN<$S>> for $S {
      fn div(scalar, matrix) -> $MatrixN<$S> { $MatrixN { $($field: scalar / matrix.$field),+ } }
    });
    impl_operator!(Rem<$MatrixN<$S>> for $S {
      fn rem(scalar, matrix) -> $MatrixN<$S> { $MatrixN { $($field: scalar % matrix.$field),+ } }
    });
  };
}

impl_as_ptr!(Mat2);
impl_as_ptr!(Mat3);
impl_as_ptr!(Mat4);
#[rustfmt::skip]
impl_matrix!(Mat2 { a1, a2, b1, b2 }, 4, mat2);
#[rustfmt::skip]
impl_matrix!(Mat3 { a1, a2, a3, b1, b2, b3, c1, c2, c3 }, 9, mat3);
#[rustfmt::skip]
impl_matrix!(Mat4 { a1, a2, a3, a4, b1, b2, b3, b4, c1, c2, c3, c4, d1, d2, d3, d4 }, 16, mat4);
#[rustfmt::skip]
impl_matrix!(Mat4x3 { a1, a2, a3, b1, b2, b3, c1, c2, c3, d1, d2, d3 }, 12, mat4x3);
#[rustfmt::skip]
impl_fixed_array_conversions!(Mat2<T> { a1: 0, a2: 1, b1: 2, b2: 0 }, 4);
#[rustfmt::skip]
impl_fixed_array_conversions!(Mat3<T> { 
  a1: 0, a2: 1, a3: 2, 
  b1: 3, b2: 4, b3: 5, 
  c1: 6, c2: 7, c3: 8 
}, 9);
#[rustfmt::skip]
impl_fixed_array_conversions!(Mat4<T> { 
  a1:  0, a2:  1, a3:  2, a4:  3, 
  b1:  4, b2:  5, b3:  6, b4:  7, 
  c1:  8, c2:  9, c3: 10, c4: 11, 
  d1: 12, d2: 13, d3: 14, d4: 15
}, 16);
#[rustfmt::skip]
impl_fixed_array_conversions!(Mat4x3<T> { 
  a1: 0, a2:  1, a3:  2,
  b1: 3, b2:  4, b3:  5,
  c1: 6, c2:  7, c3:  8,
  d1: 9, d2: 10, d3: 11
}, 12);
