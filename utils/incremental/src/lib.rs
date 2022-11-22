pub use incremental_derives::*;
use std::{any::Any, fmt::Debug};

// mod rev_ty;
mod ty;

pub use ty::*;

pub trait Incremental: Sized {
  /// `Delta` should be atomic modification unit of `Self`
  /// atomic means no invalid states between the modification
  type Delta: Clone;

  /// mutation maybe not valid and return error back.
  /// should stay valid state even if mutation failed.
  type Error: Debug;

  /// Mutator encapsulate the inner mutable state to prevent direct mutation and generate delta automatically
  /// Mutator should also direct support apply delta which constraint by MutatorApply
  ///
  /// We need this because delta could have return value.
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

pub type DeltaOf<T> = <T as Incremental>::Delta;

pub trait MutatorApply<T: Incremental> {
  fn apply(&mut self, delta: T::Delta);
}

/// Not all type can impl this kind of reversible delta
pub trait ReverseIncremental: Incremental {
  /// return reversed delta
  fn apply_rev(&mut self, delta: Self::Delta) -> Result<Self::Delta, Self::Error>;
}

/// this trait is to support incremental boxed trait object
///
/// Performance is maybe not good, each delta contains a heap allocation.
///
/// The expand method will create a lot of heap allocation? no,
/// the expand is called by delta consumer side on demand and avoid most of cost.
pub trait DynIncremental {
  fn apply(&mut self, delta: Box<dyn Any>) -> Result<(), Box<dyn Any>>;
  fn expand(&self, cb: impl FnMut(Box<dyn Any>));
}
