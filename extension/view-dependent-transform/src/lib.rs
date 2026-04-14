use std::hash::Hash;
use std::sync::Arc;

use bitflags::bitflags;
use database::*;
use fast_hash_collection::FastHashMap;
use parking_lot::Mutex;
use rendiation_algebra::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;
use serde::*;
use slab::Slab;

mod picking;
pub use picking::*;
mod occ;
pub use occ::*;
mod indirect_draw;
pub use indirect_draw::*;
mod gles_draw;
pub use gles_draw::*;

pub type ViewKey = u64;
pub type ViewSceneModelKey = (ViewKey, RawEntityHandle); // scene model

#[derive(Default, Clone)]
pub struct CurrentViewControl {
  current_view: Arc<Mutex<Option<ViewKey>>>,
}

impl CurrentViewControl {
  pub fn get(&self) -> Option<ViewKey> {
    self.current_view.lock().as_ref().copied()
  }
  pub fn set(&self, view: Option<ViewKey>) {
    *self.current_view.lock() = view;
  }
}
