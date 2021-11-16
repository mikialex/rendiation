use crate::{renderer::text::GPUxUITextPrimitive, TextInfo};
use std::collections::HashMap;

use super::GlyphCache;

#[derive(Default)]
pub struct TextCache {
  cache: HashMap<u64, GPUxUITextPrimitive>,
  queue: HashMap<u64, GPUxUITextPrimitive>,
}

impl TextCache {
  pub fn queue(&mut self, text: &TextInfo) {
    // self.text_cache.queue(text);
  }

  pub fn process_queued(&mut self, glyph_cache: &mut GlyphCache) {
    //
  }
}
