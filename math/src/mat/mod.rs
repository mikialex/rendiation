mod mat2;
mod mat3;
mod mat4;

pub use mat2::*;
pub use mat3::*;
pub use mat4::*;

use crate::*;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::ops::{AddAssign, DivAssign, MulAssign, RemAssign, SubAssign};

pub trait Matrix {}

pub trait SquareMatrix: Matrix {}

impl<T> Matrix for Mat2<T>{}
impl<T> Matrix for Mat3<T>{}
impl<T> Matrix for Mat4<T>{}

impl<T> SquareMatrix for Mat2<T>{}
impl<T> SquareMatrix for Mat3<T>{}
impl<T> SquareMatrix for Mat4<T>{}

pub struct ColumMajor<M: SquareMatrix> {
  pub mat: M,
}

pub struct RawMajor<M: SquareMatrix> {
  pub mat: M,
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

    // impl<S: BaseFloat> iter::Sum<$MatrixN<S>> for $MatrixN<S> {
    //     #[inline]
    //     fn sum<I: Iterator<Item=$MatrixN<S>>>(iter: I) -> $MatrixN<S> {
    //         iter.fold($MatrixN::zero(), Add::add)
    //     }
    // }

    // impl<'a, S: 'a + BaseFloat> iter::Sum<&'a $MatrixN<S>> for $MatrixN<S> {
    //     #[inline]
    //     fn sum<I: Iterator<Item=&'a $MatrixN<S>>>(iter: I) -> $MatrixN<S> {
    //         iter.fold($MatrixN::zero(), Add::add)
    //     }
    // }

    // impl<S: BaseFloat> iter::Product for $MatrixN<S> {
    //     #[inline]
    //     fn product<I: Iterator<Item=$MatrixN<S>>>(iter: I) -> $MatrixN<S> {
    //         iter.fold($MatrixN::identity(), Mul::mul)
    //     }
    // }

    // impl<'a, S: 'a + BaseFloat> iter::Product<&'a $MatrixN<S>> for $MatrixN<S> {
    //     #[inline]
    //     fn product<I: Iterator<Item=&'a $MatrixN<S>>>(iter: I) -> $MatrixN<S> {
    //         iter.fold($MatrixN::identity(), Mul::mul)
    //     }
    // }

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


    // impl<S: NumCast + Copy> $MatrixN<S> {
    //     /// Component-wise casting to another type
    //     #[inline]
    //     pub fn cast<T: NumCast>(&self) -> Option<$MatrixN<T>> {
    //         $(
    //             let $field = match self.$field.cast() {
    //                 Some(field) => field,
    //                 None => return None
    //             };
    //         )+
    //         Some($MatrixN { $($field),+ })
    //     }
    // }
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
impl_matrix!(Mat2{ a1, a2, b1, b2 }, 4, mat2);
impl_matrix!(Mat3{ a1, a2, a3, b1, b2, b3, c1, c2, c3 }, 9, mat3);
impl_matrix!(Mat4{ a1, a2, a3, a4, b1, b2, b3, b4, c1, c2, c3, c4, d1, d2, d3, d4 }, 16, mat4);
impl_fixed_array_conversions!(Mat2<T> { a1: 0, a2: 1, b1: 2, b2: 0 }, 4);
impl_fixed_array_conversions!(Mat3<T> { 
  a1: 0, a2: 1, a3: 2, 
  b1: 3, b2: 4, b3: 5, 
  c1: 6, c2: 7, c3: 8 
}, 9);
impl_fixed_array_conversions!(Mat4<T> { 
  a1: 0, a2: 1, a3: 2, a4: 3, 
  b1: 4, b2: 5, b3: 6, b4: 7, 
  c1: 8, c2: 9, c3:10, c4: 11, 
  d1:12, d2:13, d3:14, d4: 15 
}, 16);