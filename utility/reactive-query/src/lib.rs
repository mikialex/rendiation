#![feature(impl_trait_in_assoc_type)]

use core::{
  pin::Pin,
  task::{Context, Poll},
};
use std::any::Any;
use std::any::TypeId;
use std::ops::DerefMut;
use std::sync::Arc;
use std::{marker::PhantomData, ops::Deref};

use fast_hash_collection::FastHashMap;
use fast_hash_collection::*;
use futures::task::AtomicWaker;
use futures::{Stream, StreamExt};
use parking_lot::lock_api::RawRwLock;
use parking_lot::RwLock;
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};
pub use query::*;
use storage::IndexKeptVec;

mod delta;
pub use delta::*;

mod generic_query;
pub use generic_query::*;

mod reactive_query;
pub use reactive_query::*;

mod utility;
pub use utility::*;

mod one_many;
pub use one_many::*;

mod collective_channel;
pub use collective_channel::*;

mod registry;
pub use registry::*;

mod lock_holder;
pub use lock_holder::*;

mod mutate_target;
pub use mutate_target::*;