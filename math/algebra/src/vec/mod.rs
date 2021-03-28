pub mod dimension;
pub mod fake_hyper_vec;
pub mod normalized;
pub mod space_marker;
pub mod swizzle;
pub mod vec2;
pub mod vec3;
pub mod vec4;
pub use dimension::*;
pub use fake_hyper_vec::*;
pub use normalized::*;
pub use space_marker::*;
pub use vec2::*;
pub use vec3::*;
pub use vec4::*;

use std::ops::*;

use crate::*;

#[macro_export]
macro_rules! vector {
  ($x:expr,$y:expr) => {
    Vec2::new($x, $y)
  };
  ($x:expr,$y:expr,$z:expr) => {
    Vec3::new($x, $y, $z)
  };
  ($x:expr,$y:expr,$z:expr,$w:expr) => {
    Vec4::new($x, $y, $z, $w)
  };
}

macro_rules! impl_vector {
  ($VectorN:ident { $($field:ident),+ }, $n:expr, $constructor:ident) => {
    impl<S> $VectorN<S> {
      /// Construct a new vector, using the provided values.
      #[inline]
      pub const fn new($($field: S),+) -> $VectorN<S> {
        $VectorN { $($field),+ }
      }

      /// return the length of element
      #[inline]
      pub fn len() -> usize {
       $n
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

    impl_index_operators!($VectorN<S>, $n,  S,  usize);
    impl_index_operators!($VectorN<S>, $n, [S], std::ops::Range<usize>);
    impl_index_operators!($VectorN<S>, $n, [S], std::ops::RangeTo<usize>);
    impl_index_operators!($VectorN<S>, $n, [S], std::ops::RangeFrom<usize>);
    impl_index_operators!($VectorN<S>, $n, [S], std::ops::RangeFull);

    impl_operator!(<S> Add<$VectorN<S> > for $VectorN<S> {
      fn add(lhs, rhs) -> $VectorN<S> { $VectorN::new($(lhs.$field + rhs.$field),+) }
    });
    impl_assignment_operator!(<S> AddAssign<$VectorN<S> > for $VectorN<S> {
      fn add_assign(&mut self, other) { $(self.$field += other.$field);+ }
    });

    impl_operator!(<S> Sub<$VectorN<S> > for $VectorN<S> {
      fn sub(lhs, rhs) -> $VectorN<S> { $VectorN::new($(lhs.$field - rhs.$field),+) }
    });
    impl_assignment_operator!(<S> SubAssign<$VectorN<S> > for $VectorN<S> {
      fn sub_assign(&mut self, other) { $(self.$field -= other.$field);+ }
    });

    impl_operator!(<S> Mul<S> for $VectorN<S> {
      fn mul(vector, scalar) -> $VectorN<S> { $VectorN::new($(vector.$field * scalar),+) }
    });
    impl_operator!(<S> Mul<$VectorN<S>> for $VectorN<S> {
      fn mul(lhs, rhs) -> $VectorN<S> { $VectorN::new($(lhs.$field * rhs.$field),+) }
    });
    impl_assignment_operator!(<S> MulAssign<S> for $VectorN<S> {
      fn mul_assign(&mut self, scalar) { $(self.$field *= scalar);+ }
    });

    impl_operator!(<S> Div<S> for $VectorN<S> {
      fn div(vector, scalar) -> $VectorN<S> { $VectorN::new($(vector.$field / scalar),+) }
    });
    impl_operator!(<S> Div<$VectorN<S>> for $VectorN<S> {
      fn div(lhs, rhs) -> $VectorN<S> { $VectorN::new($(lhs.$field / rhs.$field),+) }
    });
    impl_assignment_operator!(<S> DivAssign<S> for $VectorN<S> {
      fn div_assign(&mut self, scalar) { $(self.$field /= scalar);+ }
    });

    impl_operator!(<S> Rem<S> for $VectorN<S> {
      fn rem(vector, scalar) -> $VectorN<S> { $VectorN::new($(vector.$field % scalar),+) }
    });
    impl_assignment_operator!(<S> RemAssign<S> for $VectorN<S> {
      fn rem_assign(&mut self, scalar) { $(self.$field %= scalar);+ }
    });

    impl_scalar_ops!($VectorN<usize> { $($field),+ });
    impl_scalar_ops!($VectorN<u8>    { $($field),+ });
    impl_scalar_ops!($VectorN<u16>   { $($field),+ });
    impl_scalar_ops!($VectorN<u32>   { $($field),+ });
    impl_scalar_ops!($VectorN<u64>   { $($field),+ });
    impl_scalar_ops!($VectorN<isize> { $($field),+ });
    impl_scalar_ops!($VectorN<i8>    { $($field),+ });
    impl_scalar_ops!($VectorN<i16>   { $($field),+ });
    impl_scalar_ops!($VectorN<i32>   { $($field),+ });
    impl_scalar_ops!($VectorN<i64>   { $($field),+ });
    impl_scalar_ops!($VectorN<f32>   { $($field),+ });
    impl_scalar_ops!($VectorN<f64>   { $($field),+ });

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

impl_as_ref_self!(Vec2);
impl_as_ref_self!(Vec3);
impl_as_ref_self!(Vec4);

impl_as_ptr!(Vec2);
impl_as_ptr!(Vec3);
impl_as_ptr!(Vec4);

impl_vector!(Vec2 { x, y }, 2, vec2);
impl_vector!(Vec3 { x, y, z }, 3, vec3);
impl_vector!(Vec4 { x, y, z, w }, 4, vec4);

impl_fixed_array_conversions!(Vec2<T> { x: 0, y: 1 }, 2);
impl_fixed_array_conversions!(Vec3<T> { x: 0, y: 1, z: 2 }, 3);
impl_fixed_array_conversions!(Vec4<T> { x: 0, y: 1, z: 2, w: 3 }, 4);

impl_tuple_conversions!(Vec2<T> { x, y }, (T, T));
impl_tuple_conversions!(Vec3<T> { x, y, z }, (T, T, T));
impl_tuple_conversions!(Vec4<T> { x, y, z, w }, (T, T, T, T));
