use std::{
  ffi::{c_char, CStr},
  sync::OnceLock,
};

use database::global_database;
use database_tracing::*;
pub use viewer_content_api_trace_info::RendiationCxAPITraceEvent;

#[derive(Clone)]
pub struct APITraceEventSender {
  writer: Option<FileTraceWriter<TracingMessage<RendiationCxAPITraceEvent>>>,
}

impl APITraceEventSender {
  pub fn emit(&self, event: &RendiationCxAPITraceEvent) {
    if let Some(ref writer) = self.writer {
      writer.write_message(TracingMessage::Event(event.clone()));
    }
  }
}

static API_TRACE_SENDER: OnceLock<APITraceEventSender> = OnceLock::new();

pub fn setup_tracing(trace_write_path: *const c_char) {
  let sender = if trace_write_path.is_null() {
    APITraceEventSender { writer: None }
  } else {
    let trace_write_path = unsafe { CStr::from_ptr(trace_write_path) };
    if let Ok(trace_write_path) = trace_write_path.to_str() {
      let writer =
        FileTraceWriter::<TracingMessage<RendiationCxAPITraceEvent>>::new(trace_write_path);
      let sender = writer.clone();
      start_tracing(&global_database(), writer);
      log::warn!("api tracing is enabled, extra performance overhead is expected");
      APITraceEventSender {
        writer: Some(sender),
      }
    } else {
      log::error!("unable to convert c style config path into utf8, tracing is disabled");
      APITraceEventSender { writer: None }
    }
  };
  API_TRACE_SENDER.set(sender).ok();
}

pub fn expect_tracing_event_emitter() -> APITraceEventSender {
  API_TRACE_SENDER
    .get()
    .cloned()
    .expect("expect_tracing_event_emitter: rendiation_init must be called before any api usage")
}
