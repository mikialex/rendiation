//! Type erased database access for C, see the generated `rdb.h` for the typed C++ wrapper.

use std::sync::OnceLock;

use database_ffi::*;

use crate::*;

const _: () = assert!(
  size_of::<ViewerEntityHandle>() == size_of::<FfiEntityHandle>()
    && align_of::<ViewerEntityHandle>() == align_of::<FfiEntityHandle>()
);

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdbStatus {
  Ok,
  DeadHandle,
  InvalidValue,
  UnknownKey,
}

/// the hash of the schema used by the library, compare it with the `SCHEMA_HASH` in the
/// generated header to make sure they are matched.
#[unsafe(no_mangle)]
pub extern "C" fn rdb_schema_hash() -> u64 {
  rdb_schema().hash
}

/// return an empty handle if the entity key is unknown
#[unsafe(no_mangle)]
pub extern "C" fn rdb_entity_create(entity: u64) -> ViewerEntityHandle {
  rdb_schema()
    .create_entity(&global_database(), entity)
    .map(Into::into)
    .unwrap_or_else(ViewerEntityHandle::empty)
}

#[unsafe(no_mangle)]
pub extern "C" fn rdb_entity_drop(entity: u64, handle: ViewerEntityHandle) -> RdbStatus {
  rdb_schema()
    .delete_entity(&global_database(), entity, handle.into())
    .into()
}

/// the value must point to the C layout of the component value declared in the generated header
#[unsafe(no_mangle)]
pub extern "C" fn rdb_component_write(
  component: u64,
  handle: ViewerEntityHandle,
  value: *const c_void,
) -> RdbStatus {
  if value.is_null() {
    return RdbStatus::InvalidValue;
  }
  let db = global_database();
  unsafe { rdb_schema().write_component(&db, component, handle.into(), value as *const u8) }.into()
}

/// The out must point to the C layout of the component value declared in the generated header.
///
/// If the value contains heap data (string or slice), the snapshot must not be null, it will be
/// set to the owner of the heap data (or null if nothing is allocated), the pointers in out are
/// valid until the snapshot is dropped by [rdb_snapshot_drop].
#[unsafe(no_mangle)]
pub extern "C" fn rdb_component_read(
  component: u64,
  handle: ViewerEntityHandle,
  out: *mut c_void,
  snapshot: *mut *mut RdbSnapshot,
) -> RdbStatus {
  let schema = rdb_schema();
  let Some(c) = schema.component(component) else {
    return RdbStatus::UnknownKey;
  };
  if out.is_null() || (c.has_heap_data && snapshot.is_null()) {
    return RdbStatus::InvalidValue;
  }
  let db = global_database();
  let mut arena = FfiArena::default();
  let status =
    unsafe { schema.read_component(&db, component, handle.into(), out as *mut u8, &mut arena) };
  if !snapshot.is_null() {
    let owner = if arena.is_empty() {
      std::ptr::null_mut()
    } else {
      Box::into_raw(Box::new(RdbSnapshot { _arena: arena }))
    };
    unsafe { *snapshot = owner };
  }
  status.into()
}

/// owns the heap data of a read value
pub struct RdbSnapshot {
  _arena: FfiArena,
}

/// null is allowed
#[unsafe(no_mangle)]
pub extern "C" fn rdb_snapshot_drop(snapshot: *mut RdbSnapshot) {
  if !snapshot.is_null() {
    drop(unsafe { Box::from_raw(snapshot) });
  }
}

static RDB_SCHEMA: OnceLock<FfiSchema> = OnceLock::new();

pub(crate) fn setup_rdb_schema() {
  RDB_SCHEMA
    .set(FfiSchema::from_database(&global_database()))
    .ok();
}

fn rdb_schema() -> &'static FfiSchema {
  RDB_SCHEMA
    .get()
    .expect("rdb: rendiation_init must be called before any api usage")
}

/// Generate the typed C++ header of the rdb api, see `examples/generate_rdb_header.rs`.
///
/// This setups the global database, so it must be called in a process that not use the api.
pub fn generate_rdb_header() -> String {
  setup_api_database();
  rdb_schema().generate_cpp_header(&rdb_header_options())
}

fn rdb_header_options() -> CppHeaderOptions {
  CppHeaderOptions {
    namespace: "rdb".into(),
    includes: vec!["bindings.h".into()],
    entity_handle_type: "ViewerEntityHandle".into(),
    status_type: "RdbStatus".into(),
    snapshot_type: "RdbSnapshot".into(),
  }
}

impl From<FfiStatus> for RdbStatus {
  fn from(value: FfiStatus) -> Self {
    match value {
      FfiStatus::Ok => RdbStatus::Ok,
      FfiStatus::DeadHandle => RdbStatus::DeadHandle,
      FfiStatus::InvalidValue => RdbStatus::InvalidValue,
      FfiStatus::UnknownKey => RdbStatus::UnknownKey,
    }
  }
}

#[cfg(test)]
mod test {
  use std::sync::Once;

  use super::*;

  fn setup() -> &'static FfiSchema {
    static SETUP: Once = Once::new();
    SETUP.call_once(setup_api_database);
    rdb_schema()
  }

  #[test]
  fn rdb_every_component_should_roundtrip_sample_values() {
    let schema = setup();
    let failures = verify_sample_roundtrip(&global_database(), schema);
    assert!(failures.is_empty(), "{failures:#?}");
  }
}
