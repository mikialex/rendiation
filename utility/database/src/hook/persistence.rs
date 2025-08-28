use std::fs::File;
use std::io::Write;

use futures::channel::mpsc::UnboundedSender;
use futures::StreamExt;

use crate::*;

/// all db mutation in this scope will be automatically and incrementally saved.
/// when init, if the previous saved file is found, it will be loaded or the
/// `init_for_new_persistent_scope` will be called.
pub fn use_persistent_db_scope<Cx: HooksCxLike>(
  cx: &mut Cx,
  init_for_new_persistent_scope: impl FnOnce(),
  scope: impl FnOnce(&mut Cx, &CheckPointCreator),
) {
  let (cx, persist_cx) = cx.use_plain_state(PersistentContext::default);
  let sender = persist_cx.change_sender.clone();

  use_db_scoped_staged_change(
    cx,
    |cx, cp| {
      if cx.is_creating() {
        if persist_cx.is_new_crated {
          init_for_new_persistent_scope();
          persist_cx.is_new_crated = true;
        } else {
          // todo, load from file
        }
      }

      scope(cx, cp)
    },
    move |change| {
      sender.unbounded_send(change).ok();
    },
  )
}

struct PersistentContext {
  is_new_crated: bool,
  change_sender: UnboundedSender<StagedDBScopeChange>,
}

impl Default for PersistentContext {
  fn default() -> Self {
    let assume_last_run_file_path = std::env::current_dir().unwrap().join("db_save.bin");

    let is_new_crated = !assume_last_run_file_path.exists();

    let mut file = File::create(assume_last_run_file_path).unwrap();

    let (change_sender, mut receiver) = futures::channel::mpsc::unbounded::<StagedDBScopeChange>();

    // we should get a thread pool?
    // this thread is detached, but it's ok
    std::thread::spawn(move || {
      while let Some(data) = futures::executor::block_on(receiver.next()) {
        if data.is_empty() {
          continue;
        }
        println!("write {:?}", data);
        // file.write_all(&data).unwrap();
      }
      file.flush().unwrap();
    });

    Self {
      is_new_crated,
      change_sender,
    }
  }
}
