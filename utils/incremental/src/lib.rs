pub use incremental_derives::*;
use std::{any::Any, fmt::Debug};

// mod rev_ty;
mod lens;
mod ty;

pub use lens::*;
pub use ty::*;

pub trait IncrementalBase: Sized + Send + Sync + 'static {
  /// `Delta` should be atomic modification unit of `Self`
  /// atomic means no invalid states between the modification
  ///
  /// Delta could contains multi grained layer of change to allow
  /// user modify the data in different level.
  type Delta: Clone + Send + Sync + 'static;

  /// generate sequence of delta, which could reduce into self with default value;
  /// expand should use the coarse level delta first to rebuild data. the caller could
  /// decide if should expand in finer level.
  fn expand(&self, cb: impl FnMut(Self::Delta));
}

pub trait AtomicIncremental {}
impl<T> AtomicIncremental for T where T: IncrementalBase<Delta = T> {}

pub type DeltaOf<T> = <T as IncrementalBase>::Delta;

pub enum MaybeDeltaRef<'a, T: IncrementalBase> {
  Delta(&'a T::Delta),
  All(&'a T),
}

#[derive(Clone)]
pub enum MaybeDelta<T: IncrementalBase + Send + Sync> {
  Delta(T::Delta),
  All(T),
}

pub trait ApplicableIncremental: IncrementalBase {
  /// mutation maybe not valid and return error back.
  /// should stay valid state even if mutation failed.
  type Error: Debug + Send + Sync + 'static;

  /// apply the mutations into the self
  ///
  /// construct the delta explicitly
  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error>;
}

pub trait Incremental: IncrementalBase + ApplicableIncremental {}
impl<T: IncrementalBase + ApplicableIncremental> Incremental for T {}

pub trait IncrementalMutatorHelper: IncrementalBase {
  /// Mutator encapsulate the inner mutable state to prevent direct mutation and generate delta automatically
  /// Mutator should also direct support apply delta which constraint by MutatorApply
  ///
  /// We need this because delta could have return value.
  type Mutator<'a>
  where
    Self: 'a;

  fn create_mutator<'a>(
    &'a mut self,
    collector: &'a mut dyn FnMut(Self::Delta),
  ) -> Self::Mutator<'a>;
}

pub trait CompareGenDelta: Incremental {
  fn expand_diff(&self, other: &Self, cb: impl FnMut(Self::Delta));
}

/// Not all type can impl this kind of reversible delta
pub trait ReverseIncremental: Incremental {
  /// return reversed delta
  fn apply_rev(&mut self, delta: Self::Delta) -> Result<Self::Delta, Self::Error>;
}

pub trait AnyClone: Any + dyn_clone::DynClone + Send + Sync {
  fn into_any(self: Box<Self>) -> Box<dyn Any>;
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}
dyn_clone::clone_trait_object!(AnyClone);
impl<T: Any + dyn_clone::DynClone + Send + Sync> AnyClone for T {
  fn into_any(self: Box<Self>) -> Box<dyn Any> {
    self
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
}

/// this trait is to support incremental boxed trait object
///
/// Performance is maybe not good, each delta contains a heap allocation.
///
/// The expand method will create a lot of heap allocation? no,
/// the expand is called by delta consumer side on demand and avoid most of cost.
pub trait DynIncremental {
  fn apply_dyn(&mut self, delta: Box<dyn AnyClone>) -> Result<(), Box<dyn Any>>;
  fn expand_dyn(&self, cb: &mut dyn FnMut(Box<dyn AnyClone>));
}

impl<T> DynIncremental for T
where
  T: Incremental,
  T::Delta: AnyClone,
{
  fn apply_dyn(&mut self, delta: Box<dyn AnyClone>) -> Result<(), Box<dyn Any>> {
    let delta = delta.into_any().downcast::<T::Delta>().unwrap();
    self.apply(*delta).unwrap();
    Ok(())
  }

  fn expand_dyn(&self, cb: &mut dyn FnMut(Box<dyn AnyClone>)) {
    self.expand(|d| cb(Box::new(d)))
  }
}

pub trait EnumWrap<U>: Sized {
  fn wrap(self, wrapper: impl FnOnce(Self) -> U) -> U;
}

impl<T, U> EnumWrap<U> for T {
  fn wrap(self, wrapper: impl FnOnce(Self) -> U) -> U {
    wrapper(self)
  }
}
