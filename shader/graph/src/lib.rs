#![feature(const_type_name)]
#![allow(incomplete_features)]
#![feature(local_key_cell_methods)]

pub mod layout_typed;
pub use layout_typed::*;

pub mod api;
pub mod gir;
pub mod graph;

pub use api::*;
pub use gir::*;
pub use graph::*;

pub mod traits_impl;
pub use traits_impl::*;

pub mod layout;
use std::{
  any::{Any, TypeId},
  cell::RefCell,
  hash::{Hash, Hasher},
  marker::PhantomData,
  ops::*,
};

pub use bytemuck::*;
use fast_hash_collection::*;
pub use layout::*;
pub use rendiation_algebra::*;
pub use shader_derives::*;
