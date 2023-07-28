#![feature(const_type_name)]
#![allow(incomplete_features)]
#![feature(local_key_cell_methods)]

pub mod code_gen;
pub use code_gen::*;

pub mod layout_typed;
pub use layout_typed::*;

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

pub mod layout;
use std::{
  any::{Any, TypeId},
  cell::{Cell, RefCell},
  hash::{Hash, Hasher},
  marker::PhantomData,
  ops::*,
  rc::Rc,
  sync::atomic::{AtomicUsize, Ordering},
};

use arena_graph::*;
pub use bytemuck::*;
use fast_hash_collection::*;
pub use layout::*;
pub use rendiation_algebra::*;
pub use shader_derives::*;
