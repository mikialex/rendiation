use std::{
  ffi::{c_char, CStr},
  io::{Read, Write},
  sync::OnceLock,
};

use database::global_database;
use database::RawEntityHandle;
use database_tracing::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RendiationCxAPITraceEvent {
  Render {
    surface_id: u64,
  },
  CreateSurface {
    hwnd: u64,
    hinstance: u64,
    returned_surface_id: u64,
    width: u32,
    height: u32,
  },
  ResizeSurface {
    surface_id: u64,
    width: u32,
    height: u32,
  },
  DeleteSurface {
    surface_id: u64,
  },
  SetDevicePixelRatio {
    surface_id: u64,
    device_pixel_ratio: f32,
  },
  CreatePicker {
    surface_id: u64,
  },
  DropPicker {
    surface_id: u64,
  },
  PickerPickList {
    surface_id: u64,
    x: f32,
    y: f32,
    extra_screen_space_tolerance: f32,
  },
  PickRange {
    surface_id: u64,
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    contain: bool,
    precise_intersection_test: bool,
    extra_screen_space_tolerance: f32,
  },
  DropViewer,
  CreateWorldDeriveQuery,
  DropWorldDeriveQuery,
  CreateBoundingComputer,
  DropBoundingComputer,
  SceneBoundingQuery {
    scene: RawEntityHandle,
    active_view_id: Option<u64>,
  },
}

impl TraceIO for RendiationCxAPITraceEvent {
  fn write_len(&self) -> usize {
    rmp_serde::to_vec(self).map(|b| b.len()).unwrap_or(0)
  }

  fn write(&self, w: &mut impl Write) -> std::io::Result<usize> {
    let buf =
      rmp_serde::to_vec(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let len = buf.len();
    w.write_all(&buf)?;
    Ok(len)
  }

  fn read(source: &mut impl Read) -> std::io::Result<Self> {
    let mut buf = Vec::new();
    source.read_to_end(&mut buf)?;
    rmp_serde::from_slice(&buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
  }
}

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
