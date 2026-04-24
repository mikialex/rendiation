#![feature(impl_trait_in_assoc_type)]

use core::f32;

use cosmic_text::CacheKey;
use database::*;
use fast_hash_collection::*;
use rendiation_algebra::*;
use rendiation_geometry::Box2;
use rendiation_geometry::Box3;
use rendiation_scene_core::SceneModelEntity;
use rendiation_shader_api::*;
use rendiation_texture_core::GPUBufferImage;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod data_prepare;
mod gles_data_prepare;
mod gles_draw;
mod indirect_data_prepare;
mod indirect_draw;
mod slug_shader;
use std::sync::Arc;

use data_prepare::*;
pub use data_prepare::{Text3dSceneModelLocalBounding, Text3dSlugBuffer};
use gles_data_prepare::*;
pub use gles_draw::use_text3d_gles_renderer;
use gles_draw::*;
use indirect_data_prepare::*;
pub use indirect_draw::use_text3d_indirect_renderer;
use parking_lot::RwLock;
use slug_shader::*;

pub fn register_text3d_data_model(sparse: bool) {
  global_entity_of::<SceneModelEntity>()
    .declare_sparse_foreign_key_maybe_sparse::<SceneModelText3dPayload>(sparse);

  global_database()
    .declare_entity::<Text3dEntity>()
    .declare_component::<Text3dContent>();
}

use facet::Facet;
use serde::*;
#[derive(Debug, Clone, Serialize, Deserialize, Facet, PartialEq)]
pub struct Text3dContentInfo {
  pub content: String,
  pub font_size: f32,
  /// in em
  pub line_height: f32,
  /// the real glyph size(in local space) will be font_size * scale
  pub scale: f32,
  /// if not provided, a default font will be used(the rendering may not be correct)
  pub font: Option<String>,
  pub weight: Option<u32>,
  pub color: Vec4<f32>,
  pub italic: bool,
  pub width: Option<f32>,
  pub height: Option<f32>,
  pub align: TextAlignment,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Facet, PartialEq)]
pub enum TextAlignment {
  Left,
  Center,
  Right,
}

declare_foreign_key!(SceneModelText3dPayload, SceneModelEntity, Text3dEntity);

declare_entity!(Text3dEntity);
declare_component!(
  Text3dContent,
  Text3dEntity,
  Option<ExternalRefPtr<Text3dContentInfo>>
);

pub struct FontSystem {
  system: cosmic_text::FontSystem,
  swash: cosmic_text::SwashCache,
  slug_glyph_cache: FastHashMap<CacheKey, Option<SlugGlyph>>,
}

impl FontSystem {
  pub fn new() -> Self {
    let mut system = cosmic_text::FontSystem::new();
    system.db_mut().load_system_fonts();
    Self {
      system,
      swash: cosmic_text::SwashCache::new(),
      slug_glyph_cache: Default::default(),
    }
  }

  // currently we not support unload font, this is doable
  // let font_ids = self.system.db_mut().load_font_source(data);
  // font_ids can be used to remove font faces in db
  pub fn load_font(&mut self, data: Vec<u8>) {
    self.system.db_mut().load_font_data(data);
  }

  pub(crate) fn get_computed_slug_glyph(&self, key: &CacheKey) -> Option<&SlugGlyph> {
    self.slug_glyph_cache.get(key).map(|v| v.as_ref()).flatten()
  }
}
