//! The foundational linear algebra library of rendiation.
//!
//! It provides the vector ([`Vec2`], [`Vec3`], [`Vec4`]), matrix ([`Mat2`], [`Mat3`], [`Mat4`]
//! and the WGSL style non square matrices like [`Mat4x3`]), quaternion ([`Quat`]) and the
//! projection ([`PerspectiveProjection`], [`OrthographicProjection`]) primitives.
//!
//! # Conventions
//!
//! - All matrices are column major, the field `a*` is the first column, `b*` is the second column
//!   and so on. `a1, a2, a3` are the rows of the first column. This is the same memory layout as
//!   WGSL / glam, so the types can be uploaded to the GPU directly.
//! - The matrix constructors (`Mat4::new` etc) take the arguments column by column. So the source
//!   code looks like the transposed matrix.
//! - Vectors are column vectors, transforms are applied by `matrix * vector`, so `a * b * v`
//!   applies `b` first.
//! - Rotation follows the right hand rule: a positive angle rotates counter clockwise when looking
//!   from the positive end of the rotation axis toward the origin.
//! - The default NDC is OpenGL's (z in -1 to 1), use [`NDCSpaceMapper`] to target other NDC
//!   space such as WebGPU's (z in 0 to 1).
//! - Angles are in radians unless they are wrapped by [`Deg`].
//!
//! # Scalar
//!
//! Most of the math functions are generic over [`Scalar`] which is implemented for `f32` and
//! `f64`. Float constants in generic context can be created by [`ScalarConstEval`].

#![feature(trait_alias)]
#![feature(stmt_expr_attributes)]

mod angle;
mod interpolation;
mod mat;
mod projection;
mod quat;
mod shader_aligned;
mod vec;

use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

pub use angle::*;
use facet::*;
pub use interpolation::*;
pub use mat::*;
use num_traits::*;
pub use num_traits::{One, Zero};
pub use projection::*;
use serde::*;
pub use shader_aligned::*;
pub use vec::*;

pub use self::quat::*;

#[macro_use]
mod macros;

/// The floating point scalar used in generic math code, implemented for `f32` and `f64`.
pub trait Scalar = Float
  + AsPrimitive<i64>
  + FloatConst
  + ScalarConstEval
  + Copy
  + std::fmt::Debug
  + AddAssign<Self>
  + SubAssign<Self>
  + DivAssign<Self>
  + MulAssign<Self>
  + Send
  + Sync
  + Default
  + 'static;

const PI: f32 = std::f32::consts::PI;

/// Get the bit pattern of a `f32`, used to pass float literals as const generic parameter of
/// [`ScalarConstEval::eval`].
///
/// ```
/// use rendiation_algebra::*;
/// fn threshold<T: Scalar>() -> T {
///   T::eval::<{ scalar_transmute(1e-7) }>()
/// }
/// assert_eq!(threshold::<f32>(), 1e-7);
/// ```
pub const fn scalar_transmute(value: f32) -> u32 {
  value.to_bits()
}

/// Create float constants in generic context.
///
/// The constant is always evaluated in `f32` precision and then converted to the target type.
pub trait ScalarConstEval: Sized {
  /// Create the value from the `f32` bit pattern `N`, see [`scalar_transmute`].
  fn eval<const N: u32>() -> Self;
  fn half() -> Self {
    Self::eval::<{ scalar_transmute(0.5) }>()
  }
  fn two() -> Self {
    Self::eval::<{ scalar_transmute(2.0) }>()
  }
  fn three() -> Self {
    Self::eval::<{ scalar_transmute(3.0) }>()
  }
  /// The factor that converts degrees to radians.
  fn pi_by_c180() -> Self {
    Self::eval::<{ scalar_transmute(PI / 180.0) }>()
  }
  /// The factor that converts radians to degrees.
  fn c180_by_pi() -> Self {
    Self::eval::<{ scalar_transmute(180.0 / PI) }>()
  }
  /// Compute `a / b` as float.
  fn by_usize_div(a: usize, b: usize) -> Self;
}

impl<T: From<f32>> ScalarConstEval for T {
  fn eval<const N: u32>() -> Self {
    let float = f32::from_bits(N);
    float.into()
  }
  fn by_usize_div(a: usize, b: usize) -> Self {
    ((a as f32) / (b as f32)).into()
  }
}

/// The geometric entity that can be transformed by a matrix in `D` dimensional space.
///
/// The matrix is the homogeneous transform of that space, for example [`Vec3`] in 3d space is
/// transformed by [`Mat4`].
pub trait SpaceEntity<T: Scalar, const D: usize> {
  type Matrix: SquareMatrixDimension<D>;
  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self;

  #[must_use]
  fn apply_matrix_into(&mut self, mat: Self::Matrix) -> Self
  where
    Self: Clone,
  {
    let mut applied = self.clone();
    applied.apply_matrix(mat);
    applied
  }
}

/// The [`SpaceEntity`] that is `Copy`, provides the more ergonomic `apply_matrix_into`.
pub trait SpaceEntityCopyable<T: Scalar, const D: usize>: Copy + SpaceEntity<T, D> {
  #[must_use]
  fn apply_matrix_into(&self, mat: Self::Matrix) -> Self {
    *self.clone().apply_matrix(mat)
  }
}

impl<T: Scalar, const D: usize, X: Copy + SpaceEntity<T, D>> SpaceEntityCopyable<T, D> for X {}

/// Use for define the coordinate's handiness.
#[derive(Debug, Copy, Clone)]
pub enum Handiness {
  Left,
  Right,
}

/// Should impl on target clip space.
/// The target clip space is defined by the API vendors such as OpenGL or WebGPU
pub trait NDCSpaceMapper<T: Scalar = f32>: Send + Sync + dyn_clone::DynClone + 'static {
  /// We use OpenGL's NDC range as standard, this function return the transformation matrix
  /// from the OpenGL's NDC space to it's defined NDC Space
  fn transform_from_opengl_standard_ndc(&self) -> Mat4<T>;
  fn transform_into_opengl_standard_ndc(&self) -> Mat4<T> {
    self
      .transform_from_opengl_standard_ndc()
      .inverse_or_identity()
  }
}
impl Clone for Box<dyn NDCSpaceMapper<f32> + '_> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

/// The OpenGL NDC: x, y, z all in -1 to 1.
#[derive(Copy, Clone)]
pub struct OpenGLxNDC;

impl<T: Scalar> NDCSpaceMapper<T> for OpenGLxNDC {
  /// Of course, we don't need to do transform here, so it's identity
  fn transform_from_opengl_standard_ndc(&self) -> Mat4<T> {
    Mat4::identity()
  }
}

/// The WebGPU NDC: x, y in -1 to 1, z in 0 to 1.
#[derive(Copy, Clone)]
pub struct WebGPUxNDC;

impl<T: Scalar> NDCSpaceMapper<T> for WebGPUxNDC {
  /// Remap z from -1 to 1 into 0 to 1
  fn transform_from_opengl_standard_ndc(&self) -> Mat4<T> {
    #[rustfmt::skip]
    Mat4::new(
      T::one(),  T::zero(), T::zero(), T::zero(),
      T::zero(), T::one(),  T::zero(), T::zero(),
      T::zero(), T::zero(), T::half(), T::zero(),
      T::zero(), T::zero(), T::half(), T::one(),
    )
  }
}

/// Generate `into_f64` for the `f32` version and `into_f32` for the `f64` version of a type that
/// has a `map` method.
#[macro_export]
macro_rules! f32_f64_convert {
  ($ty:tt) => {
    impl $ty<f32> {
      pub fn into_f64(self) -> $ty<f64> {
        self.map(|v| v as f64)
      }
    }
    impl $ty<f64> {
      pub fn into_f32(self) -> $ty<f32> {
        self.map(|v| v as f32)
      }
    }
  };
}

f32_f64_convert!(Vec2);
f32_f64_convert!(Vec3);
f32_f64_convert!(Vec4);
f32_f64_convert!(Mat2);
f32_f64_convert!(Mat3);
f32_f64_convert!(Mat4);
f32_f64_convert!(Quat);

pub type FunctorInner<T> = <T as Functor>::Unwrapped;
pub type FunctorMapped<T, U> = <T as Functor>::Wrapped<U>;
/// The container that can map its inner value into another type.
///
/// we should move this trait to math/geometry?
pub trait Functor {
  type Unwrapped;
  type Wrapped<B>: Functor;

  fn f_map<F, B>(self, f: F) -> Self::Wrapped<B>
  where
    F: FnMut(Self::Unwrapped) -> B;

  fn f_filter_map<F, B>(self, f: F) -> Option<Self::Wrapped<B>>
  where
    F: FnMut(Self::Unwrapped) -> Option<B>;
}

#[test]
fn const_eval() {
  assert_eq!(f32::eval::<{ scalar_transmute(1.5) }>(), 1.5);
  assert_eq!(f64::eval::<{ scalar_transmute(1.5) }>(), 1.5);
  assert_eq!(f32::half(), 0.5);
  assert_eq!(f64::two(), 2.0);
  assert_eq!(f32::three(), 3.0);
  assert_eq!(f32::by_usize_div(1, 4), 0.25);
  assert!((180.0 * f64::pi_by_c180() - std::f64::consts::PI).abs() < 1e-6);
  assert!((std::f64::consts::PI * f64::c180_by_pi() - 180.0).abs() < 1e-4);
}
