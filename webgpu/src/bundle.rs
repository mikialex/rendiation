use crate::*;

pub struct GPURenderBundle {
  raw: Rc<gpu::RenderBundle>,
}

pub struct BundleCache {
  cache: Rc<RefCell<HashMap<u64, Rc<gpu::RenderBundle>>>>,
}

pub struct BundleCacheInvalidation {
  cache_id_to_drop: u64,
  cache: BundleCache,
}

impl Drop for BundleCacheInvalidation {
  fn drop(&mut self) {
    self.cache.cache.borrow_mut().remove(&self.cache_id_to_drop);
  }
}
