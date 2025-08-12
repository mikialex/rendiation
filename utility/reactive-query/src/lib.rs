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
use storage::IndexKeptVec;

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

#[derive(Clone)]
pub struct DeltaQueryAsDataChanges<T, V>(pub T, pub std::marker::PhantomData<V>);

impl<V: CValue, T: Query<Value = ValueChange<V>>> DataChanges for DeltaQueryAsDataChanges<T, V> {
  type Key = T::Key;
  type Value = V;
  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self
      .0
      .iter_key_value()
      .filter_map(|(k, v)| v.is_removed().then_some(k))
  }
  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self
      .0
      .iter_key_value()
      .filter_map(|(k, v)| v.new_value().map(|v| (k, v.clone())))
  }

  fn has_change(&self) -> bool {
    self.0.iter_key_value().next().is_some()
  }
}
