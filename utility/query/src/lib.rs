#![feature(fn_traits)]
#![feature(unboxed_closures)]

use std::future::Future;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::{hash::Hash, ops::Deref};

use dyn_clone::DynClone;
use dyn_downcast::*;
use fast_hash_collection::*;
use storage::{Arena, IndexKeptVec, IndexReusedVec};

mod id;
pub use id::*;

mod query;
pub use query::*;

mod multi_query;
pub use multi_query::*;

mod combined;
pub use combined::*;

mod lock_holder;
pub use lock_holder::*;

/// common key that could be used in query system
pub trait CKey: Eq + Hash + CValue {}
impl<T> CKey for T where T: Eq + Hash + CValue {}

/// common value that could be used in query system
pub trait CValue: Clone + Send + Sync + std::fmt::Debug + PartialEq + 'static {}
impl<T> CValue for T where T: Clone + Send + Sync + std::fmt::Debug + PartialEq + 'static {}

#[inline(always)]
pub fn avoid_huge_debug_symbols_by_boxing_iter<'a, T: 'a>(
  iter: impl Iterator<Item = T> + 'a,
) -> impl Iterator<Item = T> + 'a {
  #[cfg(debug_assertions)]
  {
    Box::new(iter) as Box<dyn Iterator<Item = T>>
  }

  #[cfg(not(debug_assertions))]
  iter
}

/// we always box future, because
///  - not affect performance too much
///  - if we not boxing at all, even release build fails on some platform
#[inline(always)]
pub fn avoid_huge_debug_symbols_by_boxing_future<T>(
  f: impl Future<Output = T> + Send + Sync + 'static,
) -> impl Future<Output = T> + Send + Sync {
  Box::new(Box::pin(f)) as Box<dyn Future<Output = T> + Unpin + Send + Sync>
}
