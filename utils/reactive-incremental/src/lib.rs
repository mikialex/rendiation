#![feature(impl_trait_in_assoc_type)]
#![feature(return_position_impl_trait_in_trait)]

use core::{
  pin::Pin,
  task::{Context, Poll},
};
use std::{
  ops::Deref,
  sync::atomic::{AtomicU64, Ordering},
};

use dyn_downcast::*;
use futures::{Future, Stream, StreamExt};
use heap_tools::Counted;
use incremental::*;
use reactive::*;

mod single;
pub use single::*;

mod single_shared;
pub use single_shared::*;

mod group;
pub use group::*;

mod group_listen;
pub use group_listen::*;

mod relation;
pub use relation::*;

mod listen_utils;
pub use listen_utils::*;

static GLOBAL_ID: AtomicU64 = AtomicU64::new(0);

pub fn alloc_global_res_id() -> u64 {
  GLOBAL_ID.fetch_add(1, Ordering::Relaxed)
}

trait ModifyIdentityDelta<T: ApplicableIncremental> {
  fn apply(self, target: &mut IncrementalSignal<T>);
}

impl<T, X> ModifyIdentityDelta<T> for X
where
  T: ApplicableIncremental<Delta = X>,
{
  fn apply(self, target: &mut IncrementalSignal<T>) {
    target.mutate(|mut m| {
      m.modify(self);
    })
  }
}

/// A globally marked item, marked by a globally incremental u64 flag
///
/// **Any object *created since process started*** must has different id.
pub trait GlobalIdentified {
  fn guid(&self) -> u64;
}
define_dyn_trait_downcaster_static!(GlobalIdentified);

/// indicate this type is allocate in arena style, which could be linearly addressed
/// (efficient random accessible)
///
/// **Any object *living* must has different id, and id must tightly reused**.
pub trait LinearIdentified {
  fn alloc_index(&self) -> u32;
}
define_dyn_trait_downcaster_static!(LinearIdentified);

/// An wrapper struct that prevent outside directly accessing the mutable T, but have to modify it
/// through the explicit delta type. When modifying, the delta maybe checked if is really valid by
/// diffing, and the change will be collect by a internal collector
pub struct Mutating<'a, T: IncrementalBase> {
  inner: &'a mut T,
  collector: &'a mut dyn FnMut(&T::Delta),
}

impl<'a, T: IncrementalBase> Mutating<'a, T> {
  pub fn new(inner: &'a mut T, collector: &'a mut dyn FnMut(&T::Delta)) -> Self {
    Self { inner, collector }
  }
}

impl<'a, T: IncrementalBase> Deref for Mutating<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.inner
  }
}

impl<'a, T: ApplicableIncremental> Mutating<'a, T> {
  pub fn modify(&mut self, delta: T::Delta) {
    if self.inner.should_apply_hint(&delta) {
      (self.collector)(&delta);
      self.inner.apply(delta).unwrap()
    }
  }
}

impl<'a, T: IncrementalBase> Mutating<'a, T> {
  /// # Safety
  /// the mutation should be record manually, and will not triggered in the collector
  pub unsafe fn get_mut_ref(&mut self) -> &mut T {
    self.inner
  }

  /// # Safety
  /// the mutation will be not apply on original data but only triggered in the collector
  pub unsafe fn trigger_change_but_not_apply(&mut self, delta: T::Delta) {
    (self.collector)(&delta);
  }
}

pub trait GlobalIdReactiveMapping<M> {
  type ChangeStream: Stream + Unpin;
  type Ctx<'a>;

  fn build(&self, ctx: &Self::Ctx<'_>) -> (M, Self::ChangeStream);

  fn update(&self, mapped: &mut M, change: &mut Self::ChangeStream, ctx: &Self::Ctx<'_>);
}

pub trait GlobalIdReactiveSimpleMapping<M> {
  type ChangeStream: Stream + Unpin;
  type Ctx<'a>;

  fn build(&self, ctx: &Self::Ctx<'_>) -> (M, Self::ChangeStream);
}
