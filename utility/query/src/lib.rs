#![feature(fn_traits)]
#![feature(unboxed_closures)]
#![feature(lazy_type_alias)]

use std::marker::PhantomData;
use std::panic::Location;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::{hash::Hash, ops::Deref};

use dyn_clone::DynClone;
use fast_hash_collection::*;
use serde::*;
use storage::*;

mod id;
pub use id::*;

mod query;
pub use query::*;

mod delta_query;
pub use delta_query::*;

mod multi_query;
pub use multi_query::*;

mod change_query;
pub use change_query::*;

mod combined;
pub use combined::*;

mod lock_holder;
pub use lock_holder::*;

mod utility;
pub use utility::*;

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
