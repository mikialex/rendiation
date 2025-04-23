#![feature(trait_alias)]
#![feature(stmt_expr_attributes)]
#![allow(clippy::transmute_ptr_to_ptr)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::float_cmp)]
#![allow(clippy::from_over_into)]

mod angle;
mod euler;
mod interpolation;
mod mat;
mod projection;
mod quat;
mod shader_aligned;
mod vec;

use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

pub use angle::*;
pub use euler::*;
use facet::*;
pub use interpolation::*;
pub use mat::*;
pub use num_traits::Zero;
use num_traits::*;
pub use projection::*;
pub use shader_aligned::*;
pub use vec::*;

pub use self::quat::*;

#[macro_use]
mod macros;

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

#[allow(clippy::transmute_float_to_int)]
pub const fn scalar_transmute(value: f32) -> u32 {
  unsafe { std::mem::transmute(value) }
}

pub trait ScalarConstEval: Sized {
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
  fn pi_by_c180() -> Self {
    Self::eval::<{ scalar_transmute(PI / 180.0) }>()
  }
  fn c180_by_pi() -> Self {
    Self::eval::<{ scalar_transmute(180.0 / PI) }>()
  }
  fn by_usize_div(a: usize, b: usize) -> Self;
}

impl<T: From<f32>> ScalarConstEval for T {
  fn eval<const N: u32>() -> Self {
    let float: f32 = f32::from_bits(N);
    float.into()
  }
  fn by_usize_div(a: usize, b: usize) -> Self {
    ((a as f32) / (b as f32)).into()
  }
}

#[test]
fn const_eval() {
  assert_eq!(f32::eval::<{ scalar_transmute(1.5) }>(), 1.5);
  assert_eq!(f64::eval::<{ scalar_transmute(1.5) }>(), 1.5);
}

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
pub trait NDCSpaceMapper<T: Scalar>: Send + Sync + dyn_clone::DynClone + 'static {
  /// We use OpenGL's NDC range as standard, this function return the transformation matrix
  /// from the OpenGL's NDC space to it's defined NDC Space
  fn transform_from_opengl_standard_ndc(&self) -> Mat4<T>;
}
impl Clone for Box<dyn NDCSpaceMapper<f32> + '_> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

#[derive(Copy, Clone)]
pub struct OpenGLxNDC;

impl<T: Scalar> NDCSpaceMapper<T> for OpenGLxNDC {
  /// Of course we don't need transform here, so it's identity
  fn transform_from_opengl_standard_ndc(&self) -> Mat4<T> {
    Mat4::identity()
  }
}

#[derive(Copy, Clone)]
pub struct WebGPUxNDC;

impl<T: Scalar> NDCSpaceMapper<T> for WebGPUxNDC {
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
