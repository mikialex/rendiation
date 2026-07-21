use std::io::{Read, Write};

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

impl TraceReplayTarget for RendiationCxAPITraceEvent {
  fn type_discriminant() -> u32 {
    11
  }
  fn is_replay_target(&self) -> bool {
    false
  }
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

  fn read(source: &mut dyn Read) -> std::io::Result<Self> {
    let mut buf = Vec::new();
    source.read_to_end(&mut buf)?;
    rmp_serde::from_slice(&buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
  }
}
