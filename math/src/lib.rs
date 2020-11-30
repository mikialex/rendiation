#![feature(specialization)]
#![feature(const_generics)]
#![feature(negative_impls)]
#![allow(incomplete_features)]

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
