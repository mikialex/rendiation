use std::fmt::Debug;

mod mvc_prototype;
mod rev;
mod ty;

use ty::*;

pub trait IncrementAble {
  /// `Delta` should be strictly the smallest atomic modification unit of `Self`
  /// atomic means no invalid states between the modification
  type Delta;
  type Error: Debug;

  /// apply the mutations into the data
  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error>;

  /// generate sequence of delta, which could reduce into self with default value;
  fn expand(&self, cb: impl FnMut(Self::Delta));
}

pub type DeltaOf<T> = <T as IncrementAble>::Delta;

/// Not all type can impl this kind of reversible delta
pub trait ReverseIncrementAble: IncrementAble {
  /// return reversed delta
  fn apply_rev(&mut self, delta: Self::Delta) -> Result<Self::Delta, Self::Error>;
}
