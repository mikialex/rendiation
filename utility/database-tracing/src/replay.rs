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
  fn is_replay_target(&self) -> bool;
}

/// Load a trace file and build a `ReplayState`.
pub fn load_replay<T: TraceIO + TraceReplayTarget>(
  input_path: impl AsRef<std::path::Path>,
) -> std::io::Result<ReplayState> {
  let mut file = std::fs::File::open(input_path)?;
  let name_table = read_trace_file_header(&mut file)?;

  let mut records = Vec::new();
  loop {
    match TracingMessage::<T>::read(&mut file) {
      Ok(msg) => {
        let (kind, is_replay_target) = extract_kind(&msg);
        let mut summary = format_replay_summary(&msg, &name_table.names);
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
          DBFastSerializeSmallBufferOrForeignKey::Pod(buffer)
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
            DBFastSerializeSmallBufferOrForeignKey::ForeignKey(remapped)
          }
          None => {
            let mut buf = Vec::new();
            let none_val: Option<RawEntityHandle> = None;
            let _ = none_val.fast_serialize(&mut buf);
            DBFastSerializeSmallBufferOrForeignKey::Pod(SmallVec::from_slice(&buf))
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
