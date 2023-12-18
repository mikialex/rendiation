use std::{any::Any, fmt::Debug};

pub use incremental_derives::*;

mod lens;
mod ty;

pub use lens::*;
pub use ty::*;

pub trait IncrementalBase: Sized + Send + Sync + 'static {
  /// `Delta` should be atomic modification unit of `Self`
  /// atomic means no invalid state observed between the modification
  ///
  /// Delta could contains multi grained layer of change to allow
  /// user modify the data in different level.
  type Delta: Clone + Send + Sync + 'static;

  /// generate sequence of delta, which could reduce into self with default value;
  /// expand should use the coarse level delta first to rebuild data. the caller could
  /// decide if should expand in finer level.
  fn expand(&self, cb: impl FnMut(Self::Delta));

  /// return the estimation of how many times the callback passed in expand will be called
  ///
  /// this method is used in optimization for preallocation
  fn expand_size(&self) -> Option<usize> {
    None
  }

  fn expand_out(&self) -> Vec<Self::Delta> {
    let mut r = Vec::with_capacity(self.expand_size().unwrap_or(1));
    self.expand(|d| r.push(d));
    r
  }
  fn expand_push_into(&self, r: &mut Vec<Self::Delta>) {
    r.reserve(self.expand_size().unwrap_or(1));
    self.expand(|d| r.push(d));
  }
}

pub type DeltaOf<T> = <T as IncrementalBase>::Delta;

/// Not all data types could impl this because this requires us to construct the delta
/// before the mutation occurs.
pub trait ApplicableIncremental: IncrementalBase {
  /// mutation maybe not valid and return error back.
  /// should stay valid state even if mutation failed.
  type Error: Debug + Send + Sync + 'static;

  /// apply the mutations into the self
  ///
  /// construct the delta explicitly
  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error>;

  /// Return the hint that the mutation is effective
  ///
  /// The impls should check this by diffing the delta with the current data.
  ///
  /// This method has a default impl the always return true. The false positive is allowed but the
  /// false negative should never exist for logic correctness
  fn should_apply_hint(&self, _delta: &Self::Delta) -> bool {
    true
  }
}

pub trait Incremental: IncrementalBase + ApplicableIncremental {}
impl<T: IncrementalBase + ApplicableIncremental> Incremental for T {}

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
/// Will the expand method will create a lot of heap allocation?  No.
/// the expand is called by delta consumer side on demand and avoid most of cost.
pub trait DynIncremental {
  fn apply_dyn(&mut self, delta: Box<dyn AnyClone>) -> Result<(), Box<dyn Any>>;
  fn expand_dyn(&self, cb: &mut dyn FnMut(Box<dyn AnyClone>));
}

impl<T> DynIncremental for T
where
  T: ApplicableIncremental,
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

/// Helper trait for write better enum nesting code
pub trait EnumWrap<U>: Sized {
  fn wrap(self, wrapper: impl FnOnce(Self) -> U) -> U;
}

impl<T, U> EnumWrap<U> for T {
  fn wrap(self, wrapper: impl FnOnce(Self) -> U) -> U {
    wrapper(self)
  }
}

pub trait IncrementalEditing: ApplicableIncremental {
  fn expand_edit_path(&self, other: &Self, cb: impl FnMut(Self::Delta));
}

pub trait ReversibleIncremental: ApplicableIncremental {
  fn reverse_delta(&self, delta: &Self::Delta) -> Self::Delta;
  fn make_reverse_delta_pair(&self, delta: Self::Delta) -> DeltaPair<Self> {
    let inverse = self.reverse_delta(&delta);
    DeltaPair {
      forward: delta,
      inverse,
    }
  }
}

pub struct DeltaPair<T: ReversibleIncremental> {
  pub forward: T::Delta,
  pub inverse: T::Delta,
}
