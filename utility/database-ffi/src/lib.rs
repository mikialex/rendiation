//! Expose a [database::Database] to C/C++ by runtime reflection.
//!
//! Every component data type is mapped to a C layout derived from its facet [Shape]. Values are
//! converted between the Rust and the C layout at runtime (by facet-reflect), so the library only
//! needs a fixed set of type erased entry points keyed by stable hashes, while a strongly typed
//! C++ header is generated from the same schema.
//!
//! The generated header expects the following entry points to be exported by the library:
//!
//! ```c
//! uint64_t rdb_schema_hash();
//! ViewerEntityHandle rdb_entity_create(uint64_t entity);
//! RdbStatus rdb_entity_drop(uint64_t entity, ViewerEntityHandle handle);
//! RdbStatus rdb_component_write(uint64_t component, ViewerEntityHandle handle, const void *value);
//! RdbStatus rdb_component_read(uint64_t component, ViewerEntityHandle handle, void *out,
//!                              RdbSnapshot **snapshot);
//! void rdb_snapshot_drop(RdbSnapshot *snapshot);
//! ```

use std::{fmt::Write, sync::Arc};

use database::*;
use facet::{
  Def, EnumRepr, NumericType, PrimitiveType, PtrConst, Shape, StructKind, Type, UserType,
};
use facet_reflect::{Partial, Peek};
use fast_hash_collection::*;

mod layout;
pub use layout::*;

mod convert;
pub use convert::*;

mod schema;
pub use schema::*;

mod header;
pub use header::*;

mod sample;
pub use sample::*;

#[cfg(test)]
mod test;

/// The C layout of an entity handle, must match the handle type used by the header
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FfiEntityHandle {
  pub index: u32,
  pub generation: u64,
}

impl FfiEntityHandle {
  pub fn empty() -> Self {
    Self {
      index: u32::MAX,
      generation: u64::MAX,
    }
  }
}

impl From<RawEntityHandle> for FfiEntityHandle {
  fn from(value: RawEntityHandle) -> Self {
    Self {
      index: value.index(),
      generation: value.generation(),
    }
  }
}

impl From<FfiEntityHandle> for RawEntityHandle {
  fn from(value: FfiEntityHandle) -> Self {
    RawEntityHandle::create_only_for_testing_with_gen(value.index as usize, value.generation)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiStatus {
  Ok,
  DeadHandle,
  InvalidValue,
  UnknownKey,
}

#[derive(Debug, Clone)]
pub struct FfiError(pub String);

impl std::fmt::Display for FfiError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.0)
  }
}

fn ffi_err(e: impl std::fmt::Display) -> FfiError {
  FfiError(e.to_string())
}

/// FNV-1a, the result must be stable across builds and platforms because it is baked into the
/// generated header.
pub fn stable_hash(s: &str) -> u64 {
  let mut hash: u64 = 0xcbf29ce484222325;
  for b in s.bytes() {
    hash ^= b as u64;
    hash = hash.wrapping_mul(0x100000001b3);
  }
  hash
}
