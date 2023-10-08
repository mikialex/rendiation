#![feature(type_alias_impl_trait)]

use std::ops::Range;

use rendiation_algebra::*;

mod builder;
pub use builder::*;
mod builtin;
pub use builtin::*;
mod parametric;
pub use parametric::*;
mod combination;
pub use combination::*;
mod primitive;
pub use primitive::*;
