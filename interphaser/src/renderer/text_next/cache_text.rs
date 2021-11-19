use linked_hash_map::LinkedHashMap;

use crate::{renderer::text::GPUxUITextPrimitive, TextInfo};
use std::collections::HashMap;

use super::{GlyphCache, LayoutedTextGlyphs, TextGlyphLayouter};

pub type TextHash = u64;

pub struct TextCache {
  cache: LinkedHashMap<TextHash, TextCacheItem>,
  queue: HashMap<TextHash, LayoutedTextGlyphs>,
  layouter: Box<dyn TextGlyphLayouter>,
}

impl TextCache {
  pub fn new(layouter: impl TextGlyphLayouter + 'static) -> Self {
    Self {
      cache: Default::default(),
      queue: Default::default(),
      layouter: Box::new(layouter),
    }
  }
}

pub struct TextCacheItem {
  layout: LayoutedTextGlyphs,
  gpu: GPUxUITextPrimitive,
}

impl TextInfo {
  pub fn hash(&self) -> TextHash {
    todo!()
  }
}

impl TextCache {
  pub fn queue(&mut self, text: &TextInfo) {
    self.queue.insert(text.hash(), self.layouter.layout(text));
  }

  pub fn process_queued(&mut self, glyph_cache: &mut GlyphCache) {
    self.queue.iter().for_each(|(_, text)| {
      for (gly_id, ras_info) in &text.glyphs {
        glyph_cache.queue_glyph(*gly_id, *ras_info)
      }
    });
  }
}
