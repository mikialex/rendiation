use std::io::{Read, Write};

use database::*;
use fast_hash_collection::*;
use smallvec::SmallVec;

use crate::message::*;

/// Parsed view of a single trace record, ready for replay.
pub struct ParsedRecord {
  pub index: usize,
  pub summary: String,
  pub kind: RecordKind,
  /// Whether this record is a replay target boundary (user event).
  pub is_replay_target: bool,
}

pub enum RecordKind {
  EntityCreated(u32, RawEntityHandle),
  EntityDeleted(u32, RawEntityHandle),
  EntityFieldSet {
    name_id: u32,
    handle: RawEntityHandle,
    field_data: EntityFieldData,
  },
  Event,
}

/// Replay state: loaded records, position, handle mapping, and the name table.
pub struct ReplayState {
  pub records: Vec<ParsedRecord>,
  pub position: usize,
  pub names: Vec<String>,
  /// Per-entity-type handle map: EntityId → (original handle → live handle).
  /// Each entity type has its own independent handle allocator, so the same
  /// RawEntityHandle value can appear in different entity types.
  pub handle_map: FastHashMap<EntityId, FastHashMap<RawEntityHandle, RawEntityHandle>>,
}

pub trait TraceReplayTarget {
  /// Returns a u32 discriminant that identifies the concrete replay event type.
  /// This is stored in the trace file header and checked during `load_replay`
  /// to ensure the recorded event type matches the expected type.
  fn type_discriminant() -> u32;
  fn is_replay_target(&self) -> bool;
}

/// A type-erased function pointer that reads trace records from a reader.
type RecordLoader = fn(&mut dyn Read, &NameTable) -> std::io::Result<Vec<ParsedRecord>>;

/// A type-erased function pointer that converts trace records to human-readable text.
type TextConverter =
  fn(&mut dyn Read, &[String], Option<&Database>, usize, &mut dyn Write) -> std::io::Result<()>;

#[derive(Clone)]
struct ReplayTypeEntry {
  type_name: &'static str,
  loader: RecordLoader,
  text_converter: TextConverter,
}

/// Registry of [`TraceReplayTarget`] types, keyed by [`TraceReplayTarget::type_discriminant`].
///
/// Register event types with [`register`](Self::register), then load any compatible trace file
/// via [`load`](Self::load) — the correct reader is dispatched automatically based on the
/// type discriminant stored in the file header.
#[derive(Clone)]
pub struct ReplayTypeRegistry {
  entries: FastHashMap<u32, ReplayTypeEntry>,
}

impl ReplayTypeRegistry {
  pub fn new() -> Self {
    Self {
      entries: FastHashMap::default(),
    }
  }

  /// Register a replayable event type so its trace files can be loaded dynamically.
  ///
  /// The type must implement [`TraceIO`] (parsing), [`TraceReplayTarget`]
  /// (discriminant + frame boundaries), and [`Debug`] (text formatting).
  pub fn register<T: TraceIO + TraceReplayTarget + std::fmt::Debug + 'static>(&mut self) {
    let disc = T::type_discriminant();
    let entry = ReplayTypeEntry {
      type_name: std::any::type_name::<T>(),
      loader: read_records_for::<T>,
      text_converter: convert_to_text_for::<T>,
    };
    self.entries.insert(disc, entry);
  }

  /// Load a trace file by dispatching to the registered reader for the stored type discriminant.
  ///
  /// Returns an error when:
  /// * The file header is corrupt or has an unsupported version.
  /// * No type has been registered for the discriminant stored in the file.
  /// * A record fails to deserialize.
  pub fn load(&self, input_path: impl AsRef<std::path::Path>) -> std::io::Result<LoadedReplay> {
    let mut file = std::fs::File::open(input_path.as_ref())?;
    let (name_table, disc) = read_trace_file_header(&mut file)?;
    let entry = self.entries.get(&disc).ok_or_else(|| {
      std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!(
          "no replay handler registered for type discriminant {}. \
           Call ReplayTypeRegistry::register::<T>() first with the correct event type.",
          disc,
        ),
      )
    })?;
    let records = (entry.loader)(&mut file, &name_table)?;
    Ok(LoadedReplay {
      state: ReplayState {
        records,
        position: 0,
        names: name_table.names,
        handle_map: FastHashMap::default(),
      },
      type_discriminant: disc,
      type_name: entry.type_name,
    })
  }

  /// Convert a trace file to human-readable text by dispatching to the registered
  /// text converter for the stored type discriminant.
  ///
  /// See [`trace_to_text`](crate::trace_to_text) for the generic equivalent.
  pub fn convert_to_text(
    &self,
    input_path: impl AsRef<std::path::Path>,
    output: &mut impl Write,
    db: Option<&Database>,
    max_data_debug_len: usize,
  ) -> std::io::Result<()> {
    let mut file = std::fs::File::open(input_path.as_ref())?;
    let (name_table, disc) = read_trace_file_header(&mut file)?;
    let entry = self.entries.get(&disc).ok_or_else(|| {
      std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!(
          "no replay handler registered for type discriminant {}. \
           Call ReplayTypeRegistry::register::<T>() first with the correct event type.",
          disc,
        ),
      )
    })?;
    (entry.text_converter)(&mut file, &name_table.names, db, max_data_debug_len, output)
  }
}

/// The result of loading a trace file via [`ReplayTypeRegistry::load`].
pub struct LoadedReplay {
  /// The ready-to-replay state.
  pub state: ReplayState,
  /// The type discriminant from the file header (matches the registered type).
  pub type_discriminant: u32,
  /// The `std::any::type_name` of the registered event type that produced this trace.
  pub type_name: &'static str,
}

/// Read trace records from a reader that is already positioned past the header.
/// Returns a `RecordLoader` function pointer when monomorphized for a concrete `T`.
fn read_records_for<T: TraceIO + TraceReplayTarget>(
  reader: &mut dyn Read,
  name_table: &NameTable,
) -> std::io::Result<Vec<ParsedRecord>> {
  let names = &name_table.names;
  let mut records = Vec::new();
  loop {
    match TracingMessage::<T>::read(reader) {
      Ok(msg) => {
        let (kind, is_replay_target) = extract_kind(&msg);
        let mut summary = format_replay_summary(&msg, names);
        if is_replay_target {
          summary.push_str(" ◀ target");
        }
        records.push(ParsedRecord {
          index: records.len(),
          summary,
          kind,
          is_replay_target,
        });
      }
      Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
      Err(e) => return Err(e),
    }
  }
  Ok(records)
}

/// Convert trace records to human-readable text.
/// Returns a `TextConverter` function pointer when monomorphized for a concrete `T`.
fn convert_to_text_for<T: TraceIO + std::fmt::Debug>(
  reader: &mut dyn Read,
  names: &[String],
  db: Option<&Database>,
  max_data_debug_len: usize,
  output: &mut dyn Write,
) -> std::io::Result<()> {
  let ctx = crate::FormatCtx {
    names,
    db,
    max_data_debug_len,
  };
  loop {
    match TracingMessage::<T>::read(reader) {
      Ok(msg) => {
        let line = crate::format_message(&msg, &ctx);
        writeln!(output, "{}", line)?;
      }
      Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
      Err(e) => return Err(e),
    }
  }
  Ok(())
}

/// Load a trace file and build a `ReplayState`.
///
/// Validates that the stored type discriminant matches `T::type_discriminant()`.
pub fn load_replay<T: TraceIO + TraceReplayTarget>(
  input_path: impl AsRef<std::path::Path>,
) -> std::io::Result<ReplayState> {
  let mut file = std::fs::File::open(input_path)?;
  let (name_table, stored_disc) = read_trace_file_header(&mut file)?;
  let expected_disc = T::type_discriminant();
  if stored_disc != expected_disc {
    return Err(std::io::Error::new(
      std::io::ErrorKind::InvalidData,
      format!(
        "trace file type discriminant mismatch: expected {} but file has {}. \
         The trace was recorded with a different event type and cannot be replayed with the current type.",
        expected_disc, stored_disc,
      ),
    ));
  }

  let records = read_records_for::<T>(&mut file, &name_table)?;

  Ok(ReplayState {
    records,
    position: 0,
    names: name_table.names,
    handle_map: FastHashMap::default(),
  })
}

fn extract_kind<T: TraceReplayTarget>(msg: &TracingMessage<T>) -> (RecordKind, bool) {
  match msg {
    TracingMessage::Event(e) => (RecordKind::Event, e.is_replay_target()),
    TracingMessage::DatabaseMutation(db_msg) => match db_msg {
      DatabaseTracingMessage::EntityCreated(name_id, handle) => {
        (RecordKind::EntityCreated(*name_id, *handle), false)
      }
      DatabaseTracingMessage::EntityDeleted(name_id, handle) => {
        (RecordKind::EntityDeleted(*name_id, *handle), false)
      }
      DatabaseTracingMessage::EntityFieldSet(name_id, handle, field_data) => (
        RecordKind::EntityFieldSet {
          name_id: *name_id,
          handle: *handle,
          field_data: field_data.clone(),
        },
        false,
      ),
    },
  }
}

/// Apply records until the next target event (inclusive).
/// After this returns, the database is at a consistent frame boundary.
/// If no target event remains, applies all remaining records.
pub fn step_forward(state: &mut ReplayState, db: &Database) {
  loop {
    if state.position >= state.records.len() {
      break;
    }
    let is_target = state.records[state.position].is_replay_target;
    step_forward_single(state, db);
    if is_target {
      break;
    }
  }
}

/// Apply a single record at `state.position` and advance by one.
/// Use this for fine-grained inspection; prefer `step_forward` for normal replay.
pub fn step_forward_single(state: &mut ReplayState, db: &Database) {
  if state.position >= state.records.len() {
    return;
  }
  let record = &state.records[state.position];
  apply_single(db, &state.names, &mut state.handle_map, &record.kind);
  state.position += 1;
}

/// Apply a single record kind.
fn apply_single(
  db: &Database,
  names: &[String],
  handle_map: &mut FastHashMap<EntityId, FastHashMap<RawEntityHandle, RawEntityHandle>>,
  kind: &RecordKind,
) {
  match kind {
    RecordKind::EntityCreated(name_id, orig) => {
      let e_name = lookup_name(names, *name_id);
      let e_id = resolve_entity_id(db, e_name);
      let live = db.entity_writer_untyped_dyn(e_id).new_entity(|w| w);
      handle_map.entry(e_id).or_default().insert(*orig, live);
    }
    RecordKind::EntityDeleted(name_id, orig) => {
      let e_name = lookup_name(names, *name_id);
      let e_id = resolve_entity_id(db, e_name);
      if let Some(live) = handle_map.get(&e_id).and_then(|m| m.get(orig)).copied() {
        db.entity_writer_untyped_dyn(e_id).delete_entity(live);
        handle_map.get_mut(&e_id).unwrap().remove(orig);
      }
    }
    RecordKind::EntityFieldSet {
      name_id,
      handle,
      field_data,
    } => {
      apply_field_set(db, names, *name_id, field_data, *handle, handle_map);
    }
    RecordKind::Event => {}
  }
}

/// Reset and replay from 0 to `target` record index.
pub fn restart_and_run_to(state: &mut ReplayState, db: &Database, target: usize) {
  let target = target.min(state.records.len());
  state.position = 0;
  state.handle_map.clear();
  while state.position < target {
    step_forward_single(state, db);
  }
}

fn lookup_name(names: &[String], id: u32) -> &str {
  names.get(id as usize).map(|s| s.as_str()).unwrap_or("?")
}

fn resolve_entity_id(db: &Database, name: &str) -> EntityId {
  let mapping = db.name_mapping.read();
  *mapping
    .entities_inv
    .get(name)
    .unwrap_or_else(|| panic!("entity \"{}\" not found in live database", name))
}

fn apply_field_set(
  db: &Database,
  names: &[String],
  name_id: u32,
  field_data: &EntityFieldData,
  original_handle: RawEntityHandle,
  handle_map: &FastHashMap<EntityId, FastHashMap<RawEntityHandle, RawEntityHandle>>,
) {
  let component_name = lookup_name(names, name_id);

  let name_mapping = db.name_mapping.read();
  let c_id = *name_mapping
    .components_inv
    .get(component_name)
    .unwrap_or_else(|| panic!("component \"{}\" not found in live db", component_name));
  let e_id = *name_mapping
    .component_to_entity
    .get(&c_id)
    .unwrap_or_else(|| panic!("entity for component \"{}\" not found", component_name));
  drop(name_mapping);

  let live_handle = handle_map
    .get(&e_id)
    .and_then(|m| m.get(&original_handle))
    .copied()
    .unwrap_or_else(|| {
      panic!(
        "entity owning \"{}\" with original handle {:?} not created yet — invalid trace",
        component_name, original_handle,
      )
    });

  db.access_table_dyn(e_id, |table| {
    table.access_component(c_id, |component| {
      let mut writer = component.write_untyped();
      let value = match field_data {
        EntityFieldData::Pod(data) => {
          let buffer = SmallVec::from_slice(data);
          DatabaseSerializedFieldBufferOrForeignKey::Pod(buffer)
        }
        EntityFieldData::ForeignKey(fk) => match fk {
          Some(h) => {
            let target_e_id = component.as_foreign_key.unwrap_or_else(|| {
              panic!(
                "FK component \"{}\" missing foreign key target",
                component_name
              )
            });
            let remapped = handle_map
              .get(&target_e_id)
              .and_then(|m| m.get(h))
              .copied()
              .unwrap_or_else(|| panic!("FK target {:?} not created yet — invalid trace", h));
            DatabaseSerializedFieldBufferOrForeignKey::ForeignKey(remapped)
          }
          None => {
            let mut buf = Vec::new();
            let none_val: Option<RawEntityHandle> = None;
            let _ = none_val.serialize_to_writer(&mut buf);
            DatabaseSerializedFieldBufferOrForeignKey::Pod(SmallVec::from_slice(&buf))
          }
        },
      };
      unsafe {
        writer.write_by_small_serialize_data(live_handle, value);
      }
    });
  });
}

fn format_replay_summary<T>(msg: &TracingMessage<T>, names: &[String]) -> String {
  fn lookup(names: &[String], id: u32) -> &str {
    names.get(id as usize).map(|s| s.as_str()).unwrap_or("?")
  }
  match msg {
    TracingMessage::Event(_) => "[Event]".to_string(),
    TracingMessage::DatabaseMutation(db_msg) => match db_msg {
      DatabaseTracingMessage::EntityCreated(name_id, handle) => format!(
        "Created entity=\"{}\" ({}, g:{})",
        lookup(names, *name_id),
        handle.alloc_index(),
        handle.generation()
      ),
      DatabaseTracingMessage::EntityDeleted(name_id, handle) => format!(
        "Deleted entity=\"{}\" ({}, g:{})",
        lookup(names, *name_id),
        handle.alloc_index(),
        handle.generation()
      ),
      DatabaseTracingMessage::EntityFieldSet(name_id, handle, field_data) => {
        let value = match field_data {
          EntityFieldData::Pod(data) => format!("{}B", data.len()),
          EntityFieldData::ForeignKey(fk) => format!("FK={:?}", fk),
        };
        format!(
          "Set \"{}\" ({}, g:{}) {}",
          lookup(names, *name_id),
          handle.alloc_index(),
          handle.generation(),
          value
        )
      }
    },
  }
}
