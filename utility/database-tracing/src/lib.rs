mod message;
mod replay;
mod writer;

use std::fmt::Debug;

use database::*;
use fast_hash_collection::*;
pub use message::*;
pub use replay::*;
pub use writer::*;

/// Build a name table from all currently registered entity types and components
/// in the database. Entity names are assigned IDs first, then component names.
pub fn build_name_table(database: &Database) -> NameTable {
  let tables = database.tables.read();

  let mut names = Vec::new();
  let mut entity_name_to_id = FastHashMap::default();
  let mut component_name_to_id = FastHashMap::default();

  for (e_id, table) in tables.iter() {
    let e_name_id = names.len() as u32;
    names.push(table.name().to_string());
    entity_name_to_id.insert(*e_id, e_name_id);

    table.visit_components(|component| {
      let c_name_id = names.len() as u32;
      names.push(component.name.clone());
      component_name_to_id.insert(component.component_type_id, c_name_id);
    });
  }

  NameTable {
    names,
    entity_name_to_id,
    component_name_to_id,
  }
}

/// Start tracing all database mutations.
///
/// Builds a name table from registered entity types and components, writes the
/// protocol header via `writer.write_header()`, then subscribes to all database
/// change events. Each mutation is forwarded as a `TracingMessage` to the writer.
///
/// The writer determines how the header and records are transported
/// (`FileTraceWriter` writes to a file, custom implementations could
/// send over the network).
pub fn start_tracing<T: TraceIO + Send + Sync + 'static>(
  database: &Database,
  writer: impl TraceWriter<TracingMessage<T>>,
) -> impl TraceWriter<TracingMessage<T>> {
  let name_table = build_name_table(database);
  writer.write_header(&name_table);

  let tables = database.tables.read();

  let writer_ = writer.clone();
  database.entity_meta_watcher.on(|_| {
    unreachable!("tracing not support post entity define yet");
  });

  for (e_id, table) in tables.iter() {
    assert_eq!(table.living_entity_count(), 0);

    let e_id = *e_id;
    let e_name_id = name_table.entity_name_to_id[&e_id];
    let writer__ = writer_.clone();

    table.entity_watchers().on(move |change| {
      match change {
        ScopedMessage::Start => {}
        ScopedMessage::End => {}
        ScopedMessage::ReserveSpace(_size) => {}
        ScopedMessage::Message(change) => match change {
          EntityChange::NewEntityStartCreate(handle) => {
            let msg = DatabaseTracingMessage::EntityCreated(e_name_id, *handle);
            writer__.write_message(TracingMessage::DatabaseMutation(msg));
          }
          EntityChange::NewEntityCreated(_) => {}
          EntityChange::DeleteEntity(handle) => {
            let msg = DatabaseTracingMessage::EntityDeleted(e_name_id, *handle);
            writer__.write_message(TracingMessage::DatabaseMutation(msg));
          }
        },
      }
      false
    });

    let writer_ = writer.clone();

    table.component_define_watchers().on(|_| {
      unreachable!("tracing not support post component define yet");
    });

    table.visit_components(|component| {
      let writer__ = writer_.clone();
      let c_name_id = name_table.component_name_to_id[&component.component_type_id];
      let c_is_fk = component.as_foreign_key.is_some();
      component.data_watchers.on(move |change| unsafe {
        match change {
          ScopedMessage::Start => {}
          ScopedMessage::End => {}
          ScopedMessage::ReserveSpace(_size) => {}
          ScopedMessage::Message(change) => match change.change {
            ValueChange::Delta((data_ptr, dyn_ptr), _) => {
              let field_data = if c_is_fk {
                let fk = (data_ptr as *const Option<RawEntityHandle>).read();
                EntityFieldData::ForeignKey(fk)
              } else {
                let new = &*dyn_ptr as &dyn DynDataBaseDataType;
                // todo move the serialize into writer thread
                let buffer = new.serialize_into_buffer();
                EntityFieldData::Pod(buffer.to_vec())
              };
              let msg = DatabaseTracingMessage::EntityFieldSet(c_name_id, change.idx, field_data);
              writer__.write_message(TracingMessage::DatabaseMutation(msg));
            }
            ValueChange::Remove(_) => {}
          },
        };
        false
      });
    });
  }

  writer
}

/// Convert a trace binary file to human-readable text.
///
/// Reads the file header to recover the name table, then decodes each record
/// and writes a formatted text line. The `T` type parameter should match the
/// type used when tracing (typically `()` for viewer traces).
///
/// If `db` is provided, component data in `EntityFieldSet` records is
/// deserialized and formatted via each component's `binary_to_debug_string`
/// function. Entries whose data exceeds `max_data_debug_len` bytes are
/// displayed as `data_len=N` instead.
pub fn trace_to_text<T: TraceIO>(
  input_path: impl AsRef<std::path::Path>,
  output: &mut impl std::io::Write,
  db: Option<&Database>,
  max_data_debug_len: usize,
) -> std::io::Result<()> {
  let mut file = std::fs::File::open(input_path)?;
  let name_table = read_trace_file_header(&mut file)?;

  let ctx = FormatCtx {
    names: &name_table.names,
    db,
    max_data_debug_len,
  };

  loop {
    match TracingMessage::<T>::read(&mut file) {
      Ok(msg) => {
        let line = format_message(&msg, &ctx);
        writeln!(output, "{}", line)?;
      }
      Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
      Err(e) => return Err(e),
    }
  }

  Ok(())
}

struct FormatCtx<'a> {
  names: &'a [String],
  db: Option<&'a Database>,
  max_data_debug_len: usize,
}

fn format_message<T: Debug>(msg: &TracingMessage<T>, ctx: &FormatCtx) -> String {
  match msg {
    TracingMessage::Event(event) => format!("[Event] {:?}", event),
    TracingMessage::DatabaseMutation(db_msg) => format_db_msg(db_msg, ctx),
  }
}

fn format_db_msg(msg: &DatabaseTracingMessage, ctx: &FormatCtx) -> String {
  fn lookup(names: &[String], id: u32) -> &str {
    names.get(id as usize).map(|s| s.as_str()).unwrap_or("?")
  }

  match msg {
    DatabaseTracingMessage::EntityCreated(name_id, handle) => {
      format!(
        "[EntityCreated] entity=\"{}\" handle=({}, g:{})",
        lookup(ctx.names, *name_id),
        handle.alloc_index(),
        handle.generation()
      )
    }
    DatabaseTracingMessage::EntityDeleted(name_id, handle) => {
      format!(
        "[EntityDeleted] entity=\"{}\" handle=({}, g:{})",
        lookup(ctx.names, *name_id),
        handle.alloc_index(),
        handle.generation()
      )
    }
    DatabaseTracingMessage::EntityFieldSet(name_id, handle, field_data) => {
      let value_str = match field_data {
        EntityFieldData::ForeignKey(fk) => format!("FK={:?}", fk),
        EntityFieldData::Pod(data) => format_component_value(ctx, *name_id, data),
      };
      format!(
        "[EntityFieldSet] component=\"{}\" handle=({}, g:{}) {}",
        lookup(ctx.names, *name_id),
        handle.alloc_index(),
        handle.generation(),
        value_str
      )
    }
  }
}

fn format_component_value(ctx: &FormatCtx, name_id: u32, data: &[u8]) -> String {
  // skip large payloads
  if data.len() > ctx.max_data_debug_len {
    return format!("data_len={}", data.len());
  }

  let Some(db) = ctx.db else {
    return format!("data_len={}", data.len());
  };

  let component_name = match ctx.names.get(name_id as usize) {
    Some(n) => n.as_str(),
    None => return format!("data_len={}", data.len()),
  };

  // resolve component name → debug function via the live database
  let debug_fn = match resolve_component_debugger(db, component_name) {
    Ok(f) => f,
    Err(e) => {
      log::warn!("trace_to_text: {e}");
      return format!("data_len={}  ;schema mismatch: {e}", data.len());
    }
  };

  // convert Vec<u8> → DatabaseSerializedFieldBuffer
  let buffer: smallvec::SmallVec<[u8; 16]> = smallvec::SmallVec::from_slice(data);

  match debug_fn(buffer) {
    Some(debug_str) => debug_str,
    None => {
      log::warn!(
        "trace_to_text: deserialize failed for component \"{}\"",
        component_name
      );
      format!("data_len={}  ;deserialize failed", data.len())
    }
  }
}

/// Look up a component's debug-string converter by its unique name.
fn resolve_component_debugger(
  db: &Database,
  component_name: &str,
) -> Result<fn(DatabaseSerializedFieldBuffer) -> Option<String>, String> {
  let name_mapping = db.name_mapping.read();
  let c_id = *name_mapping
    .components_inv
    .get(component_name)
    .ok_or_else(|| format!("component \"{component_name}\" not found in live database"))?;
  let e_id = *name_mapping
    .component_to_entity
    .get(&c_id)
    .ok_or_else(|| format!("entity for component \"{component_name}\" not found"))?;
  let tables = db.tables.read();
  let table = tables
    .get(&e_id)
    .ok_or_else(|| format!("table for component \"{component_name}\" not found"))?;
  table
    .access_component(c_id, |c| c.binary_to_debug_string)
    .ok_or_else(|| format!("component \"{component_name}\" access failed"))
}
