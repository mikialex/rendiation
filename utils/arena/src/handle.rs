use std::{cmp, hash::Hash, marker::PhantomData};

/// An handle (and generation) into an `Arena`.
///
/// To get an `Handle`, insert an element into an `Arena`, and the `Handle` for
/// that element will be returned.
///
/// # Examples
///
/// ```
/// use generational_arena::Arena;
///
/// let mut arena = Arena::new();
/// let idx = arena.insert(123);
/// assert_eq!(arena[idx], 123);
/// ```
#[derive(Debug)]
pub struct Handle<T> {
  pub(crate) handle: usize,
  pub(crate) generation: u64,
  pub(crate) phantom: PhantomData<T>,
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
}

// https://stackoverflow.com/questions/31371027/copy-trait-and-phantomdata-should-this-really-move
impl<T> Clone for Handle<T> {
  fn clone(&self) -> Handle<T> {
    Handle {
      handle: self.handle,
      generation: self.generation,
      phantom: PhantomData,
    }
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

// // impl for Handle<Handle<T>>
// impl<T> Clone for Handle<Handle<T>> {
//   fn clone(&self) -> Handle<Handle<T>> {
//     Handle {
//       handle: self.handle,
//       generation: self.generation,
//       phantom: PhantomData,
//     }
//   }
// }

// impl<T> Copy for Handle<Handle<T>> {}

// impl<T> PartialEq for Handle<Handle<T>> {
//   fn eq(&self, other: &Self) -> bool {
//     self.handle == other.handle && self.generation == other.generation
//   }
// }

// impl<T> PartialOrd for Handle<Handle<T>> {
//   fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
//     Some(self.handle.cmp(&other.handle))
//   }
// }

// impl<T> Ord for Handle<Handle<T>> {
//   fn cmp(&self, other: &Self) -> cmp::Ordering {
//     self.handle.cmp(&other.handle)
//   }
// }

// impl<T> Eq for Handle<Handle<T>> {}

// impl<T> Hash for Handle<Handle<T>> {
//     fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//       self.handle.hash(state);
//     }
// }
