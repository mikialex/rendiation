#![feature(explicit_generic_args_with_impl_trait)]
#![feature(generic_const_exprs)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::many_single_char_names)]
#![allow(unstable_name_collisions)]
#![allow(incomplete_features)]

mod frame;
mod integrator;
mod pixel_sampler;
mod sampler;

pub use frame::*;
pub use integrator::*;
pub use pixel_sampler::*;
pub use sampler::*;

pub mod background;
pub mod light;
pub mod material;
pub mod math;
pub mod model;
pub mod shape;

pub use background::*;
pub use light::*;
pub use material::*;
pub use math::*;
pub use model::*;
pub use shape::*;

use rendiation_algebra::*;
