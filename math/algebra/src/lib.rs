#![feature(const_generics)]
#![feature(trait_alias)]
#![feature(stmt_expr_attributes)]
#![allow(incomplete_features)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::transmute_ptr_to_ptr)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

pub mod consts;
pub mod interpolation;
pub mod mat;
pub mod projection;
pub mod quat;
pub mod utils;
pub mod vec;

use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

pub use interpolation::*;
pub use mat::*;
pub use projection::*;
pub use vec::*;

pub use self::consts::*;
pub use self::quat::*;

pub use num_traits::*;

#[macro_use]
pub mod macros;

pub trait Scalar = Float
  + FloatConst
  + Half
  + Three
  + Two
  + AddAssign<Self>
  + SubAssign<Self>
  + DivAssign<Self>
  + MulAssign<Self>;

pub trait SpaceEntity<T: Scalar, const D: usize> {
  type Matrix: SquareMatrixDimension<D>;
  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self;
}

pub trait SpaceEntityCopyable<T: Scalar, const D: usize>: Copy + SpaceEntity<T, D> {
  fn apply_matrix_into(&self, mat: Self::Matrix) -> Self {
    *self.clone().apply_matrix(mat)
  }
}

impl<T: Scalar, const D: usize, X: Copy + SpaceEntity<T, D>> SpaceEntityCopyable<T, D> for X {}
