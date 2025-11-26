#![feature(alloc_layout_extra)]
#![feature(impl_trait_in_assoc_type)]

use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  sync::Arc,
};

use arena::*;
use bytemuck::*;
use dyn_clone::*;
use event_source::*;
pub use facet::*;
use fast_hash_collection::*;
use futures::{task::AtomicWaker, Stream};
use parking_lot::RwLock;
pub use query::*;
pub use query_hook::*;
use serde::*;

mod global;
pub use global::*;

mod hook;
pub use hook::*;

mod feature;
mod kernel;
mod semantic;
mod storage;

pub use feature::*;
pub use kernel::*;
pub use semantic::*;
pub use storage::*;
