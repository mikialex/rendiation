use linked_hash_map::LinkedHashMap;

use crate::{renderer::text::GPUxUITextPrimitive, TextInfo};
use std::collections::HashMap;

use super::{GlyphCache, LayoutedTextGlyphs};

pub type TextHash = u64;

#[derive(Default)]
pub struct TextCache {
  cache: LinkedHashMap<TextHash, TextCacheItem>,
  queue: HashMap<TextHash, TextCacheItem>,
}

pub struct TextCacheItem {
  layout: LayoutedTextGlyphs,
  gpu: GPUxUITextPrimitive,
}

impl TextCache {
  pub fn queue(&mut self, text: &TextInfo) {
    // self.text_cache.queue(text);
  }

  pub fn process_queued(&mut self, glyph_cache: &mut GlyphCache) {
    //
  }
}
