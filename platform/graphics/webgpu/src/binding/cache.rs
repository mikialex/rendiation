use crate::*;

#[derive(Clone)]
pub struct BindGroupCache {
  pub(crate) cache: Arc<RwLock<HashMap<u64, Arc<gpu::BindGroup>>>>,
}
impl BindGroupCache {
  pub(crate) fn new() -> Self {
    Self {
      cache: Default::default(),
    }
  }
}

pub struct BindGroupCacheInvalidation {
  pub(crate) cache_id_to_drop: u64,
  pub(crate) cache: BindGroupCache,
}

impl BindGroupCacheInvalidation {
  // note we not impl Clone for good reason
  pub fn clone_another(&self) -> Self {
    Self {
      cache_id_to_drop: self.cache_id_to_drop,
      cache: self.cache.clone(),
    }
  }
}

impl Drop for BindGroupCacheInvalidation {
  fn drop(&mut self) {
    self
      .cache
      .cache
      .write()
      .unwrap()
      .remove(&self.cache_id_to_drop);
  }
}

/// when holder dropped, all referenced bindgroup should drop
#[derive(Default, Clone)]
pub struct BindGroupResourceHolder {
  invalidation_tokens: Arc<RwLock<Vec<BindGroupCacheInvalidation>>>,
}

impl BindGroupResourceHolder {
  pub fn increase(&self, record: BindGroupCacheInvalidation) {
    self.invalidation_tokens.write().unwrap().push(record);
  }
}

#[derive(Clone, Default)]
pub struct BindGroupLayoutCache {
  pub cache: Arc<RwLock<HashMap<u64, GPUBindGroupLayout>>>,
}
