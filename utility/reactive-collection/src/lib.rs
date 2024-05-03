#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]
#![feature(min_specialization)]
#![feature(associated_type_bounds)]

use core::{
  pin::Pin,
  task::{Context, Poll},
};
use std::hash::Hash;
use std::sync::Arc;
use std::{
  marker::PhantomData,
  ops::Deref,
  sync::atomic::{AtomicU64, Ordering},
};

use fast_hash_collection::*;
use parking_lot::RwLock;

mod delta;
pub use delta::*;

mod virtual_collection;
pub use virtual_collection::*;

mod reactive_collection;
pub use reactive_collection::*;

mod self_contain;
pub use self_contain::*;

mod operator;
pub use operator::*;

mod container;
pub use container::*;

mod relation;
pub use relation::*;

mod id;
pub use id::*;

mod collection_channel;
pub use collection_channel::*;

mod registry;
pub use registry::*;
