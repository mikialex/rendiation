#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]
#![feature(min_specialization)]
#![feature(associated_type_bounds)]

use core::{
  pin::Pin,
  task::{Context, Poll},
};
use std::ops::Deref;

use futures::{Future, Stream, StreamExt};
use heap_tools::Counted;
use incremental::*;
pub use reactive_collection::*;
pub use reactive_stream::*;

mod collection_ext;
pub use collection_ext::*;

mod single;
pub use single::*;

mod single_shared;
pub use single_shared::*;

mod registry;
pub use registry::*;

mod group;
pub use group::*;

mod group_listen;
pub use group_listen::*;

mod listen_utils;
pub use listen_utils::*;

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

/// An wrapper struct that prevent outside directly accessing the mutable T, but have to modify it
/// through the explicit delta type. When modifying, the delta maybe checked if is really valid by
/// diffing, and the change will be collect by a internal collector
pub struct Mutating<'a, T: IncrementalBase> {
  inner: &'a mut T,
  collector: &'a mut dyn FnMut(&T::Delta, &T),
}

impl<'a, T: IncrementalBase> Mutating<'a, T> {
  pub fn new(inner: &'a mut T, collector: &'a mut dyn FnMut(&T::Delta, &T)) -> Self {
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
      (self.collector)(&delta, self.inner);
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
    (self.collector)(&delta, self.inner);
  }
}

pub trait GlobalIdReactiveMapping<M> {
  type ChangeStream: Stream + Unpin + 'static;
  type Ctx<'a>;

  fn build(&self, ctx: &Self::Ctx<'_>) -> (M, Self::ChangeStream);

  fn update(&self, mapped: &mut M, change: &mut Self::ChangeStream, ctx: &Self::Ctx<'_>);
}

pub trait GlobalIdReactiveSimpleMapping<M> {
  type ChangeStream: Stream + Unpin + 'static;
  type Ctx<'a>;

  fn build(&self, ctx: &Self::Ctx<'_>) -> (M, Self::ChangeStream);
}
