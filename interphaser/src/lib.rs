#![feature(stmt_expr_attributes)]
#![feature(capture_disjoint_fields)]
#![feature(generic_associated_types)]
#![feature(associated_type_bounds)]
#![feature(type_alias_impl_trait)]
#![allow(incomplete_features)]

mod core;
pub use crate::core::*;

#[macro_use]
mod composer;
pub use composer::*;

mod renderer;
pub use renderer::*;

mod components;
pub use components::*;

mod utils;
pub use utils::*;

mod app;
pub use app::*;

mod perf;
pub use perf::*;

use rendiation_color::*;
pub type Color = ColorWithAlpha<SRGBColor<f32>, f32>;
