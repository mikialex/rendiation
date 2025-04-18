#![feature(impl_trait_in_assoc_type)]
#![feature(fn_traits)]
#![feature(unboxed_closures)]

use core::{
  pin::Pin,
  task::{Context, Poll},
};
use std::any::Any;
use std::any::TypeId;
use std::future::Future;
use std::ops::DerefMut;
use std::sync::Arc;
use std::{marker::PhantomData, ops::Deref};

use fast_hash_collection::FastHashMap;
use fast_hash_collection::*;
use futures::task::AtomicWaker;
use futures::FutureExt;
use futures::{Stream, StreamExt};
use parking_lot::lock_api::RawRwLock;
use parking_lot::RwLock;
pub use query::*;
use serde::*;
use storage::IndexKeptVec;

mod generic_query;
pub use generic_query::*;

mod delta;
pub use delta::*;

mod previous_view;
pub use previous_view::*;

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

mod mutate_target;
pub use mutate_target::*;
