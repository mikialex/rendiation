#![feature(alloc_layout_extra)]
#![feature(impl_trait_in_assoc_type)]

use std::hash::Hash;
use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  ops::Deref,
  sync::Arc,
};

use arena::*;
use dyn_clone::*;
pub use facet::*;
use fast_hash_collection::*;
use parking_lot::RwLock;
use reactive::*;

mod global;
pub use global::*;

mod feature;
mod kernel;
mod semantic;
mod storage;

pub use feature::*;
pub use kernel::*;
pub use semantic::*;
pub use storage::*;
