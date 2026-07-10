use database::*;
use fast_hash_collection::*;
use smallvec::SmallVec;

use crate::message::*;

/// Parsed view of a single trace record, ready for replay.
pub struct ParsedRecord {
  pub index: usize,
  pub summary: String,
  pub kind: RecordKind,
  /// The original RawEntityHandle from the trace (needed for handle mapping).
  pub original_handle: Option<RawEntityHandle>,
}

pub enum RecordKind {
  EntityCreated(u32),
  EntityDeleted(u32),
  EntityFieldSet {
    name_id: u32,
    field_data: EntityFieldData,
  },
  Event,
}

/// Replay state: loaded records, position, handle mapping, and the name table.
pub struct ReplayState {
  pub records: Vec<ParsedRecord>,
  pub position: usize,
  names: Vec<String>,
  handle_map: FastHashMap<RawEntityHandle, RawEntityHandle>,
}

/// Load a trace file and build a `ReplayState`.
pub fn load_replay(input_path: impl AsRef<std::path::Path>) -> std::io::Result<ReplayState> {
  let mut file = std::fs::File::open(input_path)?;
  let name_table = read_trace_file_header(&mut file)?;

  let mut records = Vec::new();
  loop {
    match TracingMessage::<()>::read(&mut file) {
      Ok(msg) => {
        let (kind, original_handle) = extract_kind(&msg);
        let summary = format_replay_summary(&msg, &name_table.names);
        records.push(ParsedRecord {
          index: records.len(),
          summary,
          kind,
          original_handle,
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

fn extract_kind(msg: &TracingMessage<()>) -> (RecordKind, Option<RawEntityHandle>) {
  match msg {
    TracingMessage::Event(_) => (RecordKind::Event, None),
    TracingMessage::DatabaseMutation(db_msg) => match db_msg {
      DatabaseTracingMessage::EntityCreated(name_id, handle) => {
        (RecordKind::EntityCreated(*name_id), Some(*handle))
      }
      DatabaseTracingMessage::EntityDeleted(name_id, handle) => {
        (RecordKind::EntityDeleted(*name_id), Some(*handle))
      }
      DatabaseTracingMessage::EntityFieldSet(name_id, handle, field_data) => (
        RecordKind::EntityFieldSet {
          name_id: *name_id,
          field_data: clone_field_data(field_data),
        },
        Some(*handle),
      ),
    },
  }
}

fn clone_field_data(data: &EntityFieldData) -> EntityFieldData {
  match data {
    EntityFieldData::Pod(bytes) => EntityFieldData::Pod(bytes.clone()),
    EntityFieldData::ForeignKey(fk) => EntityFieldData::ForeignKey(*fk),
  }
}

/// Apply the record at `state.position` and advance.
pub fn step_forward(state: &mut ReplayState, db: &Database) {
  if state.position >= state.records.len() {
    return;
  }
  let record = &state.records[state.position];
  apply_single(
    db,
    &state.names,
    &mut state.handle_map,
    &record.kind,
    record.original_handle,
  );
  state.position += 1;
}

/// Apply a single record kind. Public for use by seek_to.
fn apply_single(
  db: &Database,
  names: &[String],
  handle_map: &mut FastHashMap<RawEntityHandle, RawEntityHandle>,
  kind: &RecordKind,
  original_handle: Option<RawEntityHandle>,
) {
  match kind {
    RecordKind::EntityCreated(name_id) => {
      let e_name = lookup_name(names, *name_id);
      let e_id = resolve_entity_id(db, e_name);
      let live = db.entity_writer_untyped_dyn(e_id).new_entity(|w| w);
      if let Some(orig) = original_handle {
        handle_map.insert(orig, live);
      }
    }
    RecordKind::EntityDeleted(name_id) => {
      if let Some(orig) = original_handle {
        if let Some(live) = handle_map.get(&orig).copied() {
          let e_name = lookup_name(names, *name_id);
          let e_id = resolve_entity_id(db, e_name);
          db.entity_writer_untyped_dyn(e_id).delete_entity(live);
        }
      }
    }
    RecordKind::EntityFieldSet {
      name_id,
      field_data,
    } => {
      if let Some(orig) = original_handle {
        if let Some(live) = handle_map.get(&orig).copied() {
          apply_field_set(db, names, *name_id, field_data, live, handle_map);
        }
      }
    }
    RecordKind::Event => {}
  }
}

/// Reset and replay from 0 to `target`.
pub fn seek_to(state: &mut ReplayState, db: &Database, target: usize) {
  let target = target.min(state.records.len());
  state.position = 0;
  state.handle_map.clear();
  while state.position < target {
    step_forward(state, db);
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
  live_handle: RawEntityHandle,
  handle_map: &FastHashMap<RawEntityHandle, RawEntityHandle>,
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
  let is_fk = name_mapping
    .components
    .get(&c_id)
    .map(|_| true)
    .unwrap_or(false);
  drop(name_mapping);

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
            let remapped = *handle_map
              .get(h)
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
      let _ = is_fk;
      unsafe {
        writer.write_by_small_serialize_data(live_handle, value);
      }
    });
  });
}

fn format_replay_summary(msg: &TracingMessage<()>, names: &[String]) -> String {
  fn lookup(names: &[String], id: u32) -> &str {
    names.get(id as usize).map(|s| s.as_str()).unwrap_or("?")
  }
  match msg {
    TracingMessage::Event(()) => "[Event]".to_string(),
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
