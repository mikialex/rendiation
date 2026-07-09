use std::{
  fmt::Debug,
  io::{Cursor, Read, Write},
};

use database::*;
use fast_hash_collection::*;

/// Maps entity type names and component type names to compact u32 IDs.
///
/// Entity names and component names share the same ID space (the `names` Vec).
/// Entity name IDs come first, followed by component name IDs.
pub struct NameTable {
  pub names: Vec<String>,
  pub entity_name_to_id: FastHashMap<EntityId, u32>,
  pub component_name_to_id: FastHashMap<ComponentId, u32>,
}

// message type tags in binary format
pub(crate) const TAG_EVENT: u8 = 0x00;
pub(crate) const TAG_ENTITY_CREATED: u8 = 0x01;
pub(crate) const TAG_ENTITY_DELETED: u8 = 0x02;
pub(crate) const TAG_ENTITY_FIELD_SET: u8 = 0x03;

// file header constants
pub(crate) const MAGIC: &[u8; 4] = b"RTRC";
pub(crate) const VERSION: u32 = 1;
pub(crate) const HEADER_SIZE: u32 = 16;

pub enum TracingMessage<T> {
  /// This is for storing any important message besides db changes
  Event(T),
  DatabaseMutation(DatabaseTracingMessage),
}

impl<T: Debug> Debug for TracingMessage<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TracingMessage::Event(event) => write!(f, "Event({:?})", event),
      TracingMessage::DatabaseMutation(msg) => write!(f, "{:?}", msg),
    }
  }
}

/// name_id is an index into the file's name table.
/// For EntityCreated/EntityDeleted, name_id refers to an entity type name.
/// For EntityFieldSet, name_id refers to a component type name.
pub enum DatabaseTracingMessage {
  EntityCreated(u32, RawEntityHandle),
  EntityDeleted(u32, RawEntityHandle),
  EntityFieldSet(u32, RawEntityHandle, Vec<u8>),
}

impl Debug for DatabaseTracingMessage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      DatabaseTracingMessage::EntityCreated(name_id, handle) => {
        write!(f, "EntityCreated(name_id={}, handle={})", name_id, handle)
      }
      DatabaseTracingMessage::EntityDeleted(name_id, handle) => {
        write!(f, "EntityDeleted(name_id={}, handle={})", name_id, handle)
      }
      DatabaseTracingMessage::EntityFieldSet(name_id, handle, data) => {
        write!(
          f,
          "EntityFieldSet(name_id={}, handle={}, data_len={})",
          name_id,
          handle,
          data.len()
        )
      }
    }
  }
}

impl DatabaseTracingMessage {
  fn type_tag(&self) -> u8 {
    match self {
      DatabaseTracingMessage::EntityCreated(_, _) => TAG_ENTITY_CREATED,
      DatabaseTracingMessage::EntityDeleted(_, _) => TAG_ENTITY_DELETED,
      DatabaseTracingMessage::EntityFieldSet(_, _, _) => TAG_ENTITY_FIELD_SET,
    }
  }

  fn payload_len(&self) -> usize {
    match self {
      DatabaseTracingMessage::EntityCreated(_, _) => 16, // name_id(4) + handle(12)
      DatabaseTracingMessage::EntityDeleted(_, _) => 16,
      DatabaseTracingMessage::EntityFieldSet(_, _, data) => {
        20 + data.len() // name_id(4) + handle(12) + data_len(4) + data
      }
    }
  }

  fn write_payload(&self, w: &mut impl Write) -> std::io::Result<()> {
    match self {
      DatabaseTracingMessage::EntityCreated(name_id, handle)
      | DatabaseTracingMessage::EntityDeleted(name_id, handle) => {
        write_u32_le(w, *name_id)?;
        write_raw_handle(w, *handle)?;
      }
      DatabaseTracingMessage::EntityFieldSet(name_id, handle, data) => {
        write_u32_le(w, *name_id)?;
        write_raw_handle(w, *handle)?;
        write_u32_le(w, data.len() as u32)?;
        w.write_all(data)?;
      }
    }
    Ok(())
  }
}

pub trait TraceIO: Debug {
  /// Returns the number of bytes that `write()` will produce.
  /// Must be callable before `write()`.
  fn write_len(&self) -> usize;
  /// Writes self to the writer. Returns the number of bytes written.
  fn write(&self, w: &mut impl Write) -> std::io::Result<usize>;
  /// Reads self from the reader. Consumes exactly `write_len()` bytes.
  fn read(source: &mut impl Read) -> std::io::Result<Self>
  where
    Self: Sized;
}

impl TraceIO for () {
  fn write_len(&self) -> usize {
    0
  }
  fn write(&self, _w: &mut impl Write) -> std::io::Result<usize> {
    Ok(0)
  }
  fn read(_source: &mut impl Read) -> std::io::Result<Self> {
    Ok(())
  }
}

impl TraceIO for DatabaseTracingMessage {
  fn write_len(&self) -> usize {
    // type_tag(1) + payload (no record_len framing, done by TracingMessage)
    1 + self.payload_len()
  }

  fn write(&self, w: &mut impl Write) -> std::io::Result<usize> {
    let len = 1 + self.payload_len();
    w.write_all(&[self.type_tag()])?;
    self.write_payload(w)?;
    Ok(len)
  }

  fn read(source: &mut impl Read) -> std::io::Result<Self> {
    let mut tag = [0u8; 1];
    source.read_exact(&mut tag)?;
    match tag[0] {
      TAG_ENTITY_CREATED | TAG_ENTITY_DELETED => {
        let name_id = read_u32_le(source)?;
        let idx = read_u32_le(source)?;
        let generation = read_u64_le(source)?;
        let handle = RawEntityHandle::create_only_for_testing_with_gen(idx as usize, generation);
        if tag[0] == TAG_ENTITY_CREATED {
          Ok(DatabaseTracingMessage::EntityCreated(name_id, handle))
        } else {
          Ok(DatabaseTracingMessage::EntityDeleted(name_id, handle))
        }
      }
      TAG_ENTITY_FIELD_SET => {
        let name_id = read_u32_le(source)?;
        let idx = read_u32_le(source)?;
        let generation = read_u64_le(source)?;
        let handle = RawEntityHandle::create_only_for_testing_with_gen(idx as usize, generation);
        let data_len = read_u32_le(source)? as usize;
        let mut data = vec![0u8; data_len];
        source.read_exact(&mut data)?;
        Ok(DatabaseTracingMessage::EntityFieldSet(
          name_id, handle, data,
        ))
      }
      _ => Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("unknown database message tag: {}", tag[0]),
      )),
    }
  }
}

impl<T: TraceIO> TraceIO for TracingMessage<T> {
  fn write_len(&self) -> usize {
    let inner_len = match self {
      // Event: TAG(1) + payload
      TracingMessage::Event(event) => 1 + event.write_len(),
      // DB mutation: TAG(1) + payload (already included in msg.write_len)
      TracingMessage::DatabaseMutation(msg) => msg.write_len(),
    };
    4 + inner_len
  }

  fn write(&self, w: &mut impl Write) -> std::io::Result<usize> {
    let inner_len = match self {
      TracingMessage::Event(event) => 1 + event.write_len(),
      TracingMessage::DatabaseMutation(msg) => msg.write_len(),
    };
    let total = 4 + inner_len;

    write_u32_le(w, inner_len as u32)?;
    match self {
      TracingMessage::Event(event) => {
        w.write_all(&[TAG_EVENT])?;
        event.write(w)?;
      }
      TracingMessage::DatabaseMutation(msg) => {
        msg.write(w)?;
      }
    }

    Ok(total)
  }

  fn read(source: &mut impl Read) -> std::io::Result<Self> {
    let record_len = read_u32_le(source)? as usize;
    let mut buf = vec![0u8; record_len];
    source.read_exact(&mut buf)?;
    let mut cursor = Cursor::new(buf);

    let mut tag = [0u8; 1];
    cursor.read_exact(&mut tag)?;

    match tag[0] {
      TAG_EVENT => {
        let event = T::read(&mut cursor)?;
        Ok(TracingMessage::Event(event))
      }
      _ => {
        cursor.set_position(0);
        let msg = DatabaseTracingMessage::read(&mut cursor)?;
        Ok(TracingMessage::DatabaseMutation(msg))
      }
    }
  }
}

// binary write helpers

pub(crate) fn write_u16_le(w: &mut impl Write, val: u16) -> std::io::Result<()> {
  w.write_all(&val.to_le_bytes())
}

pub(crate) fn write_u32_le(w: &mut impl Write, val: u32) -> std::io::Result<()> {
  w.write_all(&val.to_le_bytes())
}

pub(crate) fn write_u64_le(w: &mut impl Write, val: u64) -> std::io::Result<()> {
  w.write_all(&val.to_le_bytes())
}

pub(crate) fn write_raw_handle(w: &mut impl Write, handle: RawEntityHandle) -> std::io::Result<()> {
  write_u32_le(w, handle.alloc_index())?;
  write_u64_le(w, handle.generation())
}

// binary read helpers

pub(crate) fn read_u16_le(source: &mut impl Read) -> std::io::Result<u16> {
  let mut buf = [0u8; 2];
  source.read_exact(&mut buf)?;
  Ok(u16::from_le_bytes(buf))
}

pub(crate) fn read_u32_le(source: &mut impl Read) -> std::io::Result<u32> {
  let mut buf = [0u8; 4];
  source.read_exact(&mut buf)?;
  Ok(u32::from_le_bytes(buf))
}

pub(crate) fn read_u64_le(source: &mut impl Read) -> std::io::Result<u64> {
  let mut buf = [0u8; 8];
  source.read_exact(&mut buf)?;
  Ok(u64::from_le_bytes(buf))
}

// header I/O

/// Write the trace file header (magic, version, name table) to a writer.
pub fn write_trace_file_header(w: &mut impl Write, name_table: &NameTable) -> std::io::Result<()> {
  w.write_all(MAGIC)?;
  write_u32_le(w, VERSION)?;
  write_u32_le(w, HEADER_SIZE)?;
  write_u32_le(w, name_table.names.len() as u32)?;

  for name in &name_table.names {
    write_name_table_entry(w, name)?;
  }

  Ok(())
}

pub(crate) fn write_name_table_entry(w: &mut impl Write, name: &str) -> std::io::Result<()> {
  let bytes = name.as_bytes();
  write_u16_le(w, bytes.len() as u16)?;
  w.write_all(bytes)
}

/// Read and validate the trace file header. Returns the name table
/// (with empty debuggers since function pointers cannot be serialized).
pub fn read_trace_file_header(source: &mut impl Read) -> std::io::Result<NameTable> {
  let mut magic_buf = [0u8; 4];
  source.read_exact(&mut magic_buf)?;
  if &magic_buf != MAGIC {
    return Err(std::io::Error::new(
      std::io::ErrorKind::InvalidData,
      format!(
        "invalid magic: expected {:?}, got {:?}",
        std::str::from_utf8(MAGIC).unwrap(),
        std::str::from_utf8(&magic_buf).unwrap_or("<non-utf8>")
      ),
    ));
  }

  let version = read_u32_le(source)?;
  if version != VERSION {
    return Err(std::io::Error::new(
      std::io::ErrorKind::InvalidData,
      format!("unsupported version: {}", version),
    ));
  }

  let _header_len = read_u32_le(source)?;
  let name_count = read_u32_le(source)? as usize;

  let mut names = Vec::with_capacity(name_count);
  for _ in 0..name_count {
    let name_len = read_u16_le(source)? as usize;
    let mut name_buf = vec![0u8; name_len];
    source.read_exact(&mut name_buf)?;
    names.push(
      String::from_utf8(name_buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
    );
  }

  Ok(NameTable {
    names,
    entity_name_to_id: FastHashMap::default(),
    component_name_to_id: FastHashMap::default(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_handle(idx: usize, generation: u64) -> RawEntityHandle {
    RawEntityHandle::create_only_for_testing_with_gen(idx, generation)
  }

  // -- write format tests --

  #[test]
  fn test_db_msg_entity_created_binary_format() {
    let handle = make_handle(42, 5);
    let msg = DatabaseTracingMessage::EntityCreated(3, handle);

    // write_len: type_tag(1) + payload(16) = 17 (no record_len framing)
    let len = msg.write_len();
    assert_eq!(len, 17);

    let mut buf = Vec::new();
    let written = msg.write(&mut buf).unwrap();
    assert_eq!(written, len);

    assert_eq!(buf[0], TAG_ENTITY_CREATED);
    assert_eq!(&buf[1..5], &3u32.to_le_bytes());
    assert_eq!(&buf[5..9], &42u32.to_le_bytes());
    assert_eq!(&buf[9..17], &5u64.to_le_bytes());
  }

  #[test]
  fn test_db_msg_entity_deleted_binary_format() {
    let handle = make_handle(1, 100);
    let msg = DatabaseTracingMessage::EntityDeleted(7, handle);

    let mut buf = Vec::new();
    msg.write(&mut buf).unwrap();

    assert_eq!(buf[0], TAG_ENTITY_DELETED);
    assert_eq!(&buf[1..5], &7u32.to_le_bytes());
    assert_eq!(&buf[5..9], &1u32.to_le_bytes());
    assert_eq!(&buf[9..17], &100u64.to_le_bytes());
  }

  #[test]
  fn test_db_msg_entity_field_set_binary_format() {
    let handle = make_handle(0, 0);
    let data = vec![0xAA, 0xBB, 0xCC];
    let msg = DatabaseTracingMessage::EntityFieldSet(2, handle, data.clone());

    // write_len: type_tag(1) + name_id(4) + handle(12) + data_len(4) + data(3) = 24
    let len = msg.write_len();
    assert_eq!(len, 24);

    let mut buf = Vec::new();
    let written = msg.write(&mut buf).unwrap();
    assert_eq!(written, len);

    assert_eq!(buf[0], TAG_ENTITY_FIELD_SET);
    assert_eq!(&buf[1..5], &2u32.to_le_bytes());
    assert_eq!(&buf[5..9], &0u32.to_le_bytes());
    assert_eq!(&buf[9..17], &0u64.to_le_bytes());
    assert_eq!(&buf[17..21], &3u32.to_le_bytes());
    assert_eq!(&buf[21..24], &[0xAA, 0xBB, 0xCC]);
  }

  #[test]
  fn test_tracing_message_event() {
    let msg = TracingMessage::<()>::Event(());
    // record_len(4) + TAG(1) + event_body(0) = 5; inner_len = TAG(1) + body(0) = 1
    let len = msg.write_len();
    assert_eq!(len, 5);

    let mut buf = Vec::new();
    let written = msg.write(&mut buf).unwrap();
    assert_eq!(written, len);

    assert_eq!(&buf[0..4], &1u32.to_le_bytes()); // record_len = 1
    assert_eq!(buf[4], TAG_EVENT);
  }

  #[test]
  fn test_tracing_message_db_mutation_framing() {
    let handle = make_handle(10, 20);
    let db_msg = DatabaseTracingMessage::EntityCreated(0, handle);
    let db_inner_len = db_msg.write_len(); // 17
    let msg = TracingMessage::<()>::DatabaseMutation(db_msg);

    // total: record_len(4) + db_inner(17) = 21
    assert_eq!(msg.write_len(), 4 + db_inner_len);

    let mut buf = Vec::new();
    let written = msg.write(&mut buf).unwrap();
    assert_eq!(written, 4 + db_inner_len);

    // first 4 bytes = record_len = 17
    assert_eq!(&buf[0..4], &17u32.to_le_bytes());
    assert_eq!(buf[4], TAG_ENTITY_CREATED);
  }

  // -- round-trip tests --

  #[test]
  fn test_round_trip_entity_created() {
    let handle = make_handle(42, 5);
    let original = DatabaseTracingMessage::EntityCreated(3, handle);

    let mut buf = Vec::new();
    original.write(&mut buf).unwrap();

    let mut cursor = Cursor::new(buf);
    let read_back = DatabaseTracingMessage::read(&mut cursor).unwrap();

    match (&original, &read_back) {
      (
        DatabaseTracingMessage::EntityCreated(a1, h1),
        DatabaseTracingMessage::EntityCreated(a2, h2),
      ) => {
        assert_eq!(a1, a2);
        assert_eq!(h1, h2);
      }
      _ => panic!("variant mismatch"),
    }
  }

  #[test]
  fn test_round_trip_entity_deleted() {
    let handle = make_handle(1, 100);
    let original = DatabaseTracingMessage::EntityDeleted(7, handle);

    let mut buf = Vec::new();
    original.write(&mut buf).unwrap();

    let mut cursor = Cursor::new(buf);
    let read_back = DatabaseTracingMessage::read(&mut cursor).unwrap();
    assert_eq!(format!("{:?}", original), format!("{:?}", read_back));
  }

  #[test]
  fn test_round_trip_entity_field_set() {
    let handle = make_handle(0, 0);
    let data = vec![0xAA, 0xBB, 0xCC];
    let original = DatabaseTracingMessage::EntityFieldSet(2, handle, data);

    let mut buf = Vec::new();
    original.write(&mut buf).unwrap();

    let mut cursor = Cursor::new(buf);
    let read_back = DatabaseTracingMessage::read(&mut cursor).unwrap();
    assert_eq!(format!("{:?}", original), format!("{:?}", read_back));
  }

  #[test]
  fn test_round_trip_tracing_message_event() {
    let original = TracingMessage::<()>::Event(());

    let mut buf = Vec::new();
    original.write(&mut buf).unwrap();

    let mut cursor = Cursor::new(buf);
    let read_back = TracingMessage::<()>::read(&mut cursor).unwrap();
    assert_eq!(format!("{:?}", original), format!("{:?}", read_back));
  }

  #[test]
  fn test_round_trip_tracing_message_db_mutation() {
    let handle = make_handle(10, 20);
    let db_msg = DatabaseTracingMessage::EntityCreated(0, handle);
    let original = TracingMessage::<()>::DatabaseMutation(db_msg);

    let mut buf = Vec::new();
    original.write(&mut buf).unwrap();

    let mut cursor = Cursor::new(buf);
    let read_back = TracingMessage::<()>::read(&mut cursor).unwrap();
    assert_eq!(format!("{:?}", original), format!("{:?}", read_back));
  }

  // -- header I/O tests --

  #[test]
  fn test_header_round_trip() {
    let name_table = vec!["TestEntity".to_string(), "TestComponent".to_string()];

    let mut buf = Vec::new();
    write_trace_file_header(
      &mut buf,
      &NameTable {
        names: name_table.clone(),
        entity_name_to_id: FastHashMap::default(),
        component_name_to_id: FastHashMap::default(),
      },
    )
    .unwrap();

    let mut cursor = Cursor::new(buf);
    let read_back = read_trace_file_header(&mut cursor).unwrap();
    assert_eq!(name_table, read_back.names);
  }
}
