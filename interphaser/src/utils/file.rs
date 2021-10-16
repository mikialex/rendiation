use core::panic;
use std::{
  future::Future,
  pin::Pin,
  sync::{Arc, Mutex},
  task::{Context, Poll, Waker},
  thread,
};

pub struct UserSelectFile {
  shared_state: Arc<Mutex<SharedState>>,
}

struct SharedState {
  selected: Option<Option<String>>,
  waker: Option<Waker>,
}

impl UserSelectFile {
  pub fn new() -> Self {
    let shared_state = Arc::new(Mutex::new(SharedState {
      selected: None,
      waker: None,
    }));

    let thread_shared_state = shared_state.clone();
    thread::spawn(move || {
      let result = nfd::open_file_dialog(None, None).unwrap();

      let mut shared_state = thread_shared_state.lock().unwrap();

      shared_state.selected = Some(match result {
        nfd::Response::Okay(file_path) => Some(file_path),
        nfd::Response::OkayMultiple(_files) => panic!("not support multi file"),
        nfd::Response::Cancel => None,
      });

      if let Some(waker) = shared_state.waker.take() {
        waker.wake()
      }
    });

    Self { shared_state }
  }
}

impl Default for UserSelectFile {
  fn default() -> Self {
    Self::new()
  }
}

impl Future for UserSelectFile {
  type Output = Option<String>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let mut shared_state = self.shared_state.lock().unwrap();
    if let Some(result) = &shared_state.selected {
      Poll::Ready(result.clone())
    } else {
      shared_state.waker = Some(cx.waker().clone());
      Poll::Pending
    }
  }
}
