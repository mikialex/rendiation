#![feature(generic_associated_types)]
#![allow(clippy::needless_lifetimes)]

use std::fmt::Debug;

mod mvc_prototype;
// mod rev_ty;
mod ty;

use ty::*;

pub trait IncrementAble: Sized {
  /// `Delta` should be strictly the smallest atomic modification unit of `Self`
  /// atomic means no invalid states between the modification
  type Delta: Clone;
  /// mutation maybe not valid and return error back.
  /// Self should stay valid state even if mutation failed.
  type Error: Debug;

  /// mutate self through a mutator, which could generate delta automatically
  /// some state maybe not suitable to mutate by apply, because the requirement of return value
  /// the return value is handled in mutator implementation.
  ///
  /// Mutator should encapsulate the inner mutable state to prevent direct mutation
  /// without delta collect. Mutator should also direct support apply delta which constraint
  /// by MutatorApply
  type Mutator<'a>: MutatorApply<Self>
  where
    Self: 'a;
  fn create_mutator<'a>(
    &'a mut self,
    collector: &'a mut dyn FnMut(Self::Delta),
  ) -> Self::Mutator<'a>;

  /// apply the mutations into the self
  ///
  /// construct the delta explicitly
  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error>;

  /// generate sequence of delta, which could reduce into self with default value;
  fn expand(&self, cb: impl FnMut(Self::Delta));
}

pub type DeltaOf<T> = <T as IncrementAble>::Delta;

pub trait MutatorApply<T: IncrementAble> {
  fn apply(&mut self, delta: T::Delta);
}

/// Not all type can impl this kind of reversible delta
pub trait ReverseIncrementAble: IncrementAble {
  /// return reversed delta
  fn apply_rev(&mut self, delta: Self::Delta) -> Result<Self::Delta, Self::Error>;
}
