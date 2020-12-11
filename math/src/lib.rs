#![feature(specialization)]
#![feature(const_generics)]
#![feature(negative_impls)]
#![allow(incomplete_features)]
#![allow(many_single_char_names)]
#![allow(transmute_ptr_to_ptr)]
#![allow(too_many_arguments)]
#![allow(type_complexity)]

pub mod arithmetic;
pub mod consts;
pub mod dual;
pub mod interpolation;
pub mod mat;
pub mod math;
pub mod quat;
pub mod scalar;
pub mod utils;
pub mod vec;

pub mod wasm;
pub use wasm::*;

pub use arithmetic::*;
pub use interpolation::*;
pub use mat::*;
pub use math::*;
pub use scalar::*;
pub use vec::*;

pub use self::consts::*;
pub use self::dual::*;
pub use self::quat::*;

#[macro_use]
pub mod marcos;

pub trait SpaceEntity<T: Scalar, const D: usize> {
  fn apply_matrix(&mut self, mat: &SquareMatrixType<T, D>) -> &mut Self;
}
