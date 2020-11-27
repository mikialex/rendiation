#![feature(specialization)]
#![feature(const_generics)]
#![allow(incomplete_features)]

pub mod consts;
pub mod dual;
pub mod interpolation;
pub mod mat;
pub mod math;
pub mod quat;
pub mod swizzle;
pub mod utils;
pub mod vec;

pub mod wasm;
pub use wasm::*;

pub use interpolation::*;
pub use mat::*;
pub use math::*;
pub use vec::*;

pub use self::consts::*;
pub use self::dual::*;
pub use self::quat::*;

#[macro_use]
pub mod marcos;
