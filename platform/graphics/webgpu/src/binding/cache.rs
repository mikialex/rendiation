use crate::*;

#[derive(Clone)]
pub struct BindGroupCache {
  pub(crate) cache: Rc<RefCell<HashMap<u64, Rc<gpu::BindGroup>>>>,
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
    self.cache.cache.borrow_mut().remove(&self.cache_id_to_drop);
  }
}

/// when holder dropped, all referenced bindgroup should drop
#[derive(Default, Clone)]
pub struct BindGroupResourceHolder {
  invalidation_tokens: Arc<RwLock<Vec<BindGroupCacheInvalidation>>>,
}

impl BindGroupResourceHolder {
  pub fn create_pending_increase(&self) -> BindGroupResourcePendingIncrease {
    BindGroupResourcePendingIncrease {
      target: self.invalidation_tokens.clone(),
    }
  }
}

pub struct BindGroupResourcePendingIncrease {
  target: Arc<RwLock<Vec<BindGroupCacheInvalidation>>>,
}

impl BindGroupResourcePendingIncrease {
  pub fn increase(&self, record: BindGroupCacheInvalidation) {
    self.target.write().unwrap().push(record);
  }
}

#[derive(Clone, Default)]
pub struct BindGroupLayoutCache {
  pub cache: Rc<RefCell<HashMap<u64, GPUBindGroupLayout>>>,
}
