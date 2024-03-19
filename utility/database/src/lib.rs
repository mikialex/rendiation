#![feature(alloc_layout_extra)]

use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  ops::Deref,
  sync::Arc,
};

use arena::*;
use fast_hash_collection::*;
use parking_lot::RwLock;
use reactive::*;

mod global;
pub use global::*;

mod component;
mod db_entry;
mod entity;
mod feature;
mod semantic;
mod storage;

pub use component::*;
pub use db_entry::*;
pub use entity::*;
pub use feature::*;
pub use semantic::*;
pub use storage::*;
