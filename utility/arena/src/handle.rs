use std::{
  cmp,
  fmt::{Debug, Display},
  hash::Hash,
  marker::PhantomData,
};

use facet::*;

// use facet::*;
use crate::{Arena, Entry};

#[derive(Facet)]
pub struct Handle<T> {
  pub(crate) handle: usize,
  pub(crate) generation: u64,
  pub(crate) phantom: PhantomData<T>,
}

unsafe impl<T> Send for Handle<T> {}
unsafe impl<T> Sync for Handle<T> {}

impl<T> Display for Handle<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "({}, g:{})", self.handle, self.generation)
  }
}

impl<T> Debug for Handle<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Handle")
      .field("handle", &self.handle)
      .field("generation", &self.generation)
      .field("phantom", &self.phantom)
      .finish()
  }
}

impl<T> Handle<T> {
  /// ## Safety
  ///
  /// force type conversion
  pub unsafe fn cast_type<U>(&self) -> Handle<U> {
    let t: &Handle<U> = std::mem::transmute(self);
    *t
  }

  /// Create a new `Handle` from its raw parts.
  ///
  /// The parts must have been returned from an earlier call to
  /// `into_raw_parts`.
  ///
  /// Providing arbitrary values will lead to malformed indices and ultimately
  /// panics.
  pub fn from_raw_parts(a: usize, b: u64) -> Handle<T> {
    Handle {
      handle: a,
      generation: b,
      phantom: PhantomData,
    }
  }

  /// Convert this `Handle` into its raw parts.
  ///
  /// This niche method is useful for converting an `Handle` into another
  /// identifier type. Usually, you should prefer a newtype wrapper around
  /// `Handle` like `pub struct MyIdentifier(Handle);`.  However, for external
  /// types whose definition you can't customize, but which you can construct
  /// instances of, this method can be useful.
  pub fn into_raw_parts(self) -> (usize, u64) {
    (self.handle, self.generation)
  }

  pub fn index(self) -> usize {
    self.handle
  }
}

// https://stackoverflow.com/questions/31371027/copy-trait-and-phantomdata-should-this-really-move
impl<T> Clone for Handle<T> {
  fn clone(&self) -> Handle<T> {
    *self
  }
}

impl<T> Copy for Handle<T> {}

impl<T> PartialEq for Handle<T> {
  fn eq(&self, other: &Self) -> bool {
    self.handle == other.handle && self.generation == other.generation
  }
}

impl<T> PartialOrd for Handle<T> {
  fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
    Some(self.handle.cmp(&other.handle))
  }
}

impl<T> Ord for Handle<T> {
  fn cmp(&self, other: &Self) -> cmp::Ordering {
    self.handle.cmp(&other.handle)
  }
}

impl<T> Eq for Handle<T> {}

impl<T> Hash for Handle<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.handle.hash(state);
  }
}

impl<T> Arena<T> {
  /// Get the given position's alive handle, if the given position out of bounds or do not
  /// have alive value, the None will be returned
  pub fn get_handle(&self, index: usize) -> Option<Handle<T>> {
    match self.items.get(index) {
      Some(Entry::Occupied { generation, .. }) => Handle {
        handle: index,
        generation: *generation,
        phantom: PhantomData,
      }
      .into(),
      _ => None,
    }
  }
}
