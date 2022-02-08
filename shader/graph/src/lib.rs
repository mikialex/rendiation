use arena_graph::*;

pub use shader_derives::*;

use std::{cell::Cell, rc::Rc};

pub mod code_gen;
pub use code_gen::*;

pub mod api;
pub mod gir;
pub mod graph;
pub mod link;

pub use api::*;
pub use gir::*;
pub use graph::*;
pub use link::*;

pub mod traits_impl;
pub use traits_impl::*;

use rendiation_algebra::*;

#[cfg(test)]
mod test;
