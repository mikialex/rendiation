use crate::*;

pub struct TextCache {
  cache: Rc<RefCell<TextCachePool>>,
  layouter: Box<dyn TextGlyphLayouter>,
  glyph_cache: GlyphCache,
}

impl TextCache {
  pub fn new(glyph_cache: GlyphCache, layouter: impl TextGlyphLayouter + 'static) -> Self {
    Self {
      cache: Default::default(),
      layouter: Box::new(layouter),
      glyph_cache,
    }
  }
}

#[derive(Default)]
pub struct TextCachePool {
  /// this cache maintained the all text cache
  layout_cache: FastHashMap<TextHash, Rc<LayoutedTextGlyphs>>,

  /// this cache maintained the all text has already generated gpu vertex buffer.
  gpu_cache: FastHashSet<TextHash>,
  gpu_cache_to_delete: Vec<TextHash>,

  /// this cache maintained the all text need rendered in this frame
  queue: FastHashSet<TextHash>,
}

#[derive(Clone)]
pub struct TextLayoutRef {
  hash: TextHash,
  cache: Rc<LayoutedTextGlyphs>,
  pool: Rc<RefCell<TextCachePool>>,
}

impl TextLayoutRef {
  pub fn hash(&self) -> TextHash {
    self.hash
  }

  pub fn layout(&self) -> &LayoutedTextGlyphs {
    self.cache.as_ref()
  }
}

impl Drop for TextLayoutRef {
  fn drop(&mut self) {
    // which means no same shared.
    if Rc::strong_count(&self.cache) == 2 {
      let mut pool = self.pool.borrow_mut();

      pool.layout_cache.remove(&self.hash);
      pool.gpu_cache.remove(&self.hash);
      pool.queue.remove(&self.hash);
      pool.gpu_cache_to_delete.push(self.hash);
    }
  }
}

impl TextCache {
  pub fn queue(&mut self, hash: TextHash) {
    let mut pool = self.cache.borrow_mut();

    if !pool.gpu_cache.contains(&hash) {
      pool.queue.insert(hash);
    }
  }

  pub fn measure_size(&self, text: &TextRelaxedInfo, fonts: &FontManager) -> (f32, f32) {
    let free_unbound = TextInfo {
      content: text.content.clone(),
      bounds: (100000., 100000.),
      line_wrap: Default::default(),
      horizon_align: Default::default(),
      vertical_align: Default::default(),
      color: (0., 0., 0., 1.).into(),
      font_size: text.font_size,
      x: 0.,
      y: 0.,
    };
    let layout = self.layouter.layout(&free_unbound, fonts);

    layout
      .bound
      .map(|bound| (bound.width() + 3., bound.height())) // todo why add extra 3, the tolerance is too high
      .unwrap_or((0., 0.))
  }

  pub fn cache_layout(&mut self, text: &TextInfo, fonts: &FontManager) -> TextLayoutRef {
    let hash = text.hash();

    let mut pool = self.cache.borrow_mut();
    let layout = pool
      .layout_cache
      .entry(hash)
      .or_insert_with(|| Rc::new(self.layouter.layout(text, fonts)));

    TextLayoutRef {
      hash,
      cache: layout.clone(),
      pool: self.cache.clone(),
    }
  }

  pub fn process_queued(
    &mut self,
    fonts: &FontManager,
    tex_cache_update: impl FnMut(TextureCacheAction) -> bool, // return if cache_resize success
    mut vert_cache_update: impl FnMut(VertexCacheAction),
  ) {
    let mut pool = self.cache.borrow_mut();
    let pool: &mut TextCachePool = &mut pool;

    pool.queue.iter().for_each(|hash| {
      let layout = pool.layout_cache.get(hash).unwrap();
      for (gly_id, ras_info, _) in &layout.glyphs {
        self.glyph_cache.queue_glyph(*gly_id, *ras_info)
      }
    });

    for hash in pool.gpu_cache_to_delete.drain(..) {
      vert_cache_update(VertexCacheAction::Remove(hash))
    }

    match self
      .glyph_cache
      .process_queued(tex_cache_update, fonts)
      .unwrap()
    {
      CacheQueuedResult::Adding => {
        // build only new queued text
        for hash in pool.queue.drain() {
          let text = pool.layout_cache.get(&hash).unwrap();
          vert_cache_update(VertexCacheAction::Add {
            hash,
            data: text.generate_gpu_vertex(&self.glyph_cache),
          });
          pool.gpu_cache.insert(hash);
        }
      }
      CacheQueuedResult::Reordering => {
        // refresh all cached text with new glyph position
        for hash in pool.queue.drain() {
          pool.gpu_cache.insert(hash);
        }
        for &hash in pool.gpu_cache.iter() {
          let text = pool.layout_cache.get(&hash).unwrap();
          vert_cache_update(VertexCacheAction::Add {
            hash,
            data: text.generate_gpu_vertex(&self.glyph_cache),
          });
        }
      }
    }
  }
}

pub enum VertexCacheAction {
  Add {
    hash: TextHash,
    data: Vec<TextQuadInstance>,
  },
  Remove(TextHash),
}
