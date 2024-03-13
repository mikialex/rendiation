use dyn_downcast::*;

use crate::*;

static GLOBAL_ID: AtomicU64 = AtomicU64::new(0);

pub fn alloc_global_res_id() -> u64 {
  GLOBAL_ID.fetch_add(1, Ordering::Relaxed)
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
/// the max index should be u32::MAX - 1 (this should be sufficient for any container), we use
/// u32::MAX to represent None case to reduce memory overhead of Option<u32>
///
/// **Any object *living* must has different id, and id must tightly reused**.
pub trait LinearIdentified {
  fn alloc_index(&self) -> u32;
}
define_dyn_trait_downcaster_static!(LinearIdentified);

pub trait LinearIdentification: LinearIdentified + Copy {
  fn from_alloc_index(idx: u32) -> Self;
}

pub struct AllocIdx<T> {
  pub index: u32,
  phantom: PhantomData<T>,
}

unsafe impl<T> Send for AllocIdx<T> {}
unsafe impl<T> Sync for AllocIdx<T> {}

impl<T> std::fmt::Debug for AllocIdx<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("AllocIdx").field(&self.index).finish()
  }
}

impl<T> Clone for AllocIdx<T> {
  fn clone(&self) -> Self {
    *self
  }
}
impl<T> Copy for AllocIdx<T> {}
impl<T> PartialEq for AllocIdx<T> {
  fn eq(&self, other: &Self) -> bool {
    self.index == other.index
  }
}
impl<T> Eq for AllocIdx<T> {}
impl<T> std::hash::Hash for AllocIdx<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.index.hash(state);
  }
}

impl<T> LinearIdentified for AllocIdx<T> {
  fn alloc_index(&self) -> u32 {
    self.index
  }
}
impl<T> LinearIdentification for AllocIdx<T> {
  fn from_alloc_index(idx: u32) -> Self {
    Self::from(idx)
  }
}

impl<T> From<u32> for AllocIdx<T> {
  fn from(value: u32) -> Self {
    Self {
      index: value,
      phantom: PhantomData,
    }
  }
}
