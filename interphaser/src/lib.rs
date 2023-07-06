#![feature(stmt_expr_attributes)]
#![feature(associated_type_bounds)]
#![feature(type_alias_impl_trait)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::single_match)]
#![allow(clippy::disallowed_types)]

use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use instant::Instant;

mod core;
pub use winit;

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
pub use fontext::*;
pub use perf::*;
pub use rendiation_canvas::*;
