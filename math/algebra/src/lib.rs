#![feature(adt_const_params)]
#![feature(trait_alias)]
#![feature(stmt_expr_attributes)]
#![allow(incomplete_features)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::transmute_ptr_to_ptr)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::suspicious_operation_groupings)]
#![allow(clippy::float_cmp)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::from_over_into)]

pub mod interpolation;
pub mod mat;
pub mod projection;
pub mod quat;
pub mod std140;
pub mod vec;

use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

pub use interpolation::*;
pub use mat::*;
pub use projection::*;
pub use std140::*;
pub use vec::*;

pub use self::quat::*;

pub use num_traits::*;

#[macro_use]
pub mod macros;

pub trait Scalar = Float
  + FloatConst
  + ScalarConstEval
  + Copy
  + std::fmt::Debug
  + AddAssign<Self>
  + SubAssign<Self>
  + DivAssign<Self>
  + MulAssign<Self>;

const PI: f32 = std::f32::consts::PI;

pub trait ScalarConstEval: Sized {
  fn eval<const N: f32>() -> Self;
  fn half() -> Self {
    Self::eval::<0.5>()
  }
  fn two() -> Self {
    Self::eval::<2.0>()
  }
  fn three() -> Self {
    Self::eval::<3.0>()
  }
  fn pi_by_c180() -> Self {
    Self::eval::<{ PI / 180.0 }>()
  }
  fn c180_by_pi() -> Self {
    Self::eval::<{ 180.0 / PI }>()
  }
  fn by_usize_div(a: usize, b: usize) -> Self;
}

impl<T: From<f32>> ScalarConstEval for T {
  fn eval<const N: f32>() -> Self {
    N.into()
  }
  fn by_usize_div(a: usize, b: usize) -> Self {
    ((a as f32) / (b as f32)).into()
  }
}

#[test]
fn const_eval() {
  assert_eq!(f32::eval::<1.5>(), 1.5);
  assert_eq!(f64::eval::<1.5>(), 1.5);
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

// pub trait Chirality { }
// pub struct Left;
// pub struct Right;

pub trait NDCSpaceMapper {
  fn from_opengl_standard<T: Scalar>() -> Mat4<T>;
}

pub struct OpenGL;

impl NDCSpaceMapper for OpenGL {
  fn from_opengl_standard<T: Scalar>() -> Mat4<T> {
    Mat4::identity()
  }
}

pub struct WebGPU;

impl NDCSpaceMapper for WebGPU {
  fn from_opengl_standard<T: Scalar>() -> Mat4<T> {
    #[rustfmt::skip]
    Mat4::new(
      T::one(),  T::zero(), T::zero(), T::zero(),
      T::zero(), T::one(),  T::zero(), T::zero(),
      T::zero(), T::zero(), T::half(), T::zero(),
      T::zero(), T::zero(), T::half(), T::one(),
    )
  }
}
