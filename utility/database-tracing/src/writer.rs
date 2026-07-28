use std::{
  fs::{File, OpenOptions},
  io::Write,
  path::Path,
  sync::{Arc, Mutex},
};

use futures::StreamExt;
use futures::channel::mpsc::UnboundedSender;

use crate::message::*;

pub trait TraceWriter<T>: Send + Sync + 'static + Clone {
  /// Write the protocol-specific header.
  /// Called once before any `write_message` calls.
  fn write_header(&self, name_table: &NameTable, type_discriminant: u32);
  fn write_message(&self, message: T);
}

pub struct FileTraceWriter<T> {
  sender: UnboundedSender<T>,
  file: Arc<Mutex<File>>,
}

impl<T> Clone for FileTraceWriter<T> {
  fn clone(&self) -> Self {
    Self {
      sender: self.sender.clone(),
      file: self.file.clone(),
    }
  }
}

impl<T: TraceIO + Send + Sync + 'static> FileTraceWriter<T> {
  /// Open a trace file for writing. The file is truncated if it exists.
  /// The header is written via `TraceWriter::write_header` (called by `start_tracing`).
  pub fn new(write_file_path: impl AsRef<Path>) -> Self {
    let file = OpenOptions::new()
      .write(true)
      .create(true)
      .truncate(true)
      .open(write_file_path)
      .unwrap();
    // belt-and-suspenders: ensure file is empty even if O_TRUNC fails on some platform
    file.set_len(0).unwrap();

    let file = Arc::new(Mutex::new(file));
    let file_clone = file.clone();

    let (sender, mut receiver) = futures::channel::mpsc::unbounded::<T>();

    std::thread::spawn(move || {
      while let Some(data) = futures::executor::block_on(receiver.next()) {
        data.write(&mut *file_clone.lock().unwrap()).unwrap();
      }
      file_clone.lock().unwrap().flush().unwrap();
    });

    FileTraceWriter { sender, file }
  }
}

impl<T: Send + Sync + 'static> TraceWriter<T> for FileTraceWriter<T> {
  fn write_header(&self, name_table: &NameTable, type_discriminant: u32) {
    write_trace_file_header(
      &mut *self.file.lock().unwrap(),
      name_table,
      type_discriminant,
    )
    .unwrap();
  }

  fn write_message(&self, message: T) {
    self.sender.unbounded_send(message).unwrap();
  }
}

#[cfg(test)]
mod tests {
  use std::io::Read;

  use fast_hash_collection::FastHashMap;

  use super::*;

  #[test]
  fn test_truncate_on_new() {
    let tmp = std::env::temp_dir().join("trace_test_truncate.bin");

    // write some old content
    {
      let mut f = std::fs::File::create(&tmp).unwrap();
      f.write_all(b"old garbage data that should be gone")
        .unwrap();
    }
    assert!(std::fs::metadata(&tmp).unwrap().len() > 10);

    // create writer — should truncate, then write header
    let name_table = NameTable {
      names: vec!["TestEntity".into()],
      entity_name_to_id: FastHashMap::default(),
      component_name_to_id: FastHashMap::default(),
    };
    let writer = FileTraceWriter::<TracingMessage<()>>::new(&tmp);
    writer.write_header(&name_table, 0);

    // give background thread time
    std::thread::sleep(std::time::Duration::from_millis(50));

    // verify file starts with magic (truncated, not old garbage)
    let mut f = std::fs::File::open(&tmp).unwrap();
    let mut buf = [0u8; 4];
    f.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, MAGIC);

    // file should be small (only header), not the old garbage
    let actual_len = std::fs::metadata(&tmp).unwrap().len();
    assert!(
      actual_len < 50,
      "file should be truncated, got len {actual_len}"
    );

    std::fs::remove_file(&tmp).ok();
  }
}
