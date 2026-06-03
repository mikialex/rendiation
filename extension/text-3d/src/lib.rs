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
mod pick;
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
pub use pick::{use_text_picker, TextPicker};
use slug_shader::*;

pub fn register_text3d_data_model(sparse: bool) {
  global_entity_of::<SceneModelEntity>()
    .declare_sparse_foreign_key_maybe_sparse::<SceneModelText3dPayload>(sparse);

  global_database()
    .declare_entity::<Text3dEntity>()
    .declare_component::<Text3dLocalTransform>()
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
  /// if not provided, a default font will be used(the rendering may not be correct)
  pub font: Option<String>,
  pub weight: Option<u32>,
  pub color: Vec4<f32>,
  pub italic: bool,
  pub underline: bool,
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
declare_component!(
  Text3dLocalTransform,
  Text3dEntity,
  Mat4<f32>,
  Mat4::identity()
);

#[derive(Debug)]
pub struct TextQueryResult {
  /// note, this bbox is not considering the local transform([Text3dLocalTransform])
  pub local_bbox: Box3<f32>,
  pub cap_a_height: f32,
  pub units_per_em: u32,
}

pub fn compute_text_layout_info(
  text: RawEntityHandle,
  font_sys: &mut FontSystem,
) -> Option<TextQueryResult> {
  let text_3d = get_db_view::<Text3dContent>().access(&text)??;

  let slug = create_slug_buffer_from_text3d_content(font_sys, &text_3d);
  let local_bbox = slug.compute_local_bounding(font_sys, Mat4::identity());

  let mut cap_a_height = 0.0;
  let mut units_per_em = 0;

  if let Some(font_id) = font_sys.query_font_id(&text_3d) {
    if let Some(font) = font_sys.system.get_font(font_id, cosmic_text::Weight(400)) {
      let font = ttf_parser::Face::parse(&font.data(), 0).expect("failed to parse font");

      let glyph_id = font.glyph_index('A').expect("failed to get glyph id");

      let bbox = font
        .glyph_bounding_box(glyph_id)
        .expect("failed to get glyph bbox");
      units_per_em = font.units_per_em() as u32;
      cap_a_height = bbox.y_max as f32;
    } else {
      log::warn!("failed to get font metrics");
    };
  }

  TextQueryResult {
    local_bbox,
    units_per_em,
    cap_a_height,
  }
  .into()
}

pub struct FontSystem {
  system: cosmic_text::FontSystem,
  swash: cosmic_text::SwashCache,
  slug_glyph_cache: FastHashMap<GlyphKey, Option<SlugGlyph>>,
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

  pub(crate) fn get_computed_slug_glyph(&self, key: &GlyphKey) -> Option<&SlugGlyph> {
    self.slug_glyph_cache.get(key).map(|v| v.as_ref()).flatten()
  }

  pub fn query_font_id(&self, info: &Text3dContentInfo) -> Option<cosmic_text::fontdb::ID> {
    let font = info.font.as_ref()?;
    let style = if info.italic {
      cosmic_text::Style::Italic
    } else {
      cosmic_text::Style::Normal
    };

    let weight = cosmic_text::Weight(info.weight.unwrap_or(400) as u16);

    self.system.db().query(&cosmic_text::fontdb::Query {
      families: &[cosmic_text::Family::Name(&font)],
      weight,
      stretch: cosmic_text::Stretch::Normal,
      style,
    })
  }
}
