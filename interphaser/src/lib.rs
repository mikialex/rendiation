#![feature(stmt_expr_attributes)]
#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]
#![feature(const_option)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::single_match)]

use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use instant::Instant;

mod core;
pub use winit;

pub use crate::core::*;

mod composer;
pub use composer::*;

mod renderer;
pub use renderer::*;

mod components;
pub use components::*;

mod utils;
pub use utils::*;

mod window;
pub use window::*;

mod app;
use std::sync::Arc;

use ::core::{
  pin::Pin,
  task::{Context, Poll, Waker},
};
pub use app::*;
use fast_hash_collection::*;
pub use fontext::*;
use futures::stream::FusedStream;
use futures::Stream;
use futures::StreamExt;
use reactive::*;
use rendiation_algebra::*;
pub use rendiation_canvas_core::*;
use rendiation_webgpu::*;
use winit::event::Event;
