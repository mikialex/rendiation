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
  fn into_alloc_index(self) -> u32 {
    self.alloc_index()
  }
  fn from_alloc_index(idx: u32) -> Self;
}

impl LinearIdentified for u32 {
  fn alloc_index(&self) -> u32 {
    *self
  }
}
impl LinearIdentification for u32 {
  fn from_alloc_index(idx: u32) -> Self {
    idx
  }
}
