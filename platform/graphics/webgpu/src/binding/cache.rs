use std::sync::atomic::AtomicU64;

use crate::*;

type BindgroupHashKey = u64;
type ViewId = usize;
type BindGroupCacheId = u64;

/// If enabled, the bindgroup cache hit is decided by comparing the full key(layout id and all
/// view ids) rather than only the hash, which prevents the hash collision returns a wrong
/// bindgroup. The compare cost is small because the key is just a few ids.
pub const BINDGROUP_CACHE_FULL_KEY_COMPARE: bool = true;

struct CachedBindGroup {
  hash: BindgroupHashKey,
  /// unique id to identify the entry, the hash can not be used because it may collide
  id: BindGroupCacheId,
  layout_id: u64,
  view_ids: Vec<ViewId>,
  bindgroup: gpu::BindGroup,
  _counter: Counted<gpu::BindGroup>,
}

/// Key point of the cache control logic:
/// - bindgroup and resource_view is many-to-many relation.
/// - resource_view drop triggers all related bindgroup drop.
/// - bindgroup itself never been triggered drop.
// todo, merge per item allocation into single one.
#[derive(Default)]
pub struct BindGroupCacheInternal {
  bindgroups: HashTable<CachedBindGroup>,
  next_id: BindGroupCacheId,
  // todo, fix potential O(n) remove
  resource_views_bindgroups: FastHashMap<ViewId, Vec<(BindgroupHashKey, BindGroupCacheId)>>,
}

impl BindGroupCacheInternal {
  pub fn cached_binding_count(&self) -> usize {
    self.bindgroups.len()
  }

  pub fn clear(&mut self) {
    self.bindgroups.clear();
    self.resource_views_bindgroups.clear();
  }

  /// the hash must be computed from the layout_id and view ids
  pub fn get_or_create(
    &mut self,
    hash: BindgroupHashKey,
    layout_id: u64,
    view_ids: impl Iterator<Item = ViewId> + Clone,
    create: impl FnOnce() -> gpu::BindGroup,
  ) -> &gpu::BindGroup {
    let is_match = |cached: &CachedBindGroup| {
      cached.hash == hash
        && (!BINDGROUP_CACHE_FULL_KEY_COMPARE
          || (cached.layout_id == layout_id
            && cached.view_ids.iter().copied().eq(view_ids.clone())))
    };

    let entry = self
      .bindgroups
      .entry(hash, is_match, |cached| cached.hash)
      .or_insert_with(|| {
        let id = self.next_id;
        self.next_id += 1;
        let view_ids: Vec<_> = view_ids.clone().collect();
        for view_id in &view_ids {
          self
            .resource_views_bindgroups
            .entry(*view_id)
            .or_default()
            .push((hash, id));
        }
        CachedBindGroup {
          hash,
          id,
          layout_id,
          view_ids,
          bindgroup: create(),
          _counter: Default::default(),
        }
      });

    &entry.into_mut().bindgroup
  }

  pub fn notify_view_drop(&mut self, view_id: ViewId) {
    if let Some(all_referenced_bindings) = self.resource_views_bindgroups.remove(&view_id) {
      for (hash, id) in all_referenced_bindings {
        // none is possible because we allow cache clear
        if let Ok(entry) = self.bindgroups.find_entry(hash, |cached| cached.id == id) {
          let (removed, _) = entry.remove();
          for view_id in removed.view_ids {
            if let Some(bindings) = self.resource_views_bindgroups.get_mut(&view_id) {
              bindings
                .iter()
                .position(|v| v.1 == id)
                .map(|v| bindings.swap_remove(v));
              if bindings.is_empty() {
                self.resource_views_bindgroups.remove(&view_id);
              }
            }
          }
        }
      }
    }
  }
}

#[derive(Clone, Default)]
pub struct BindGroupCache {
  pub(crate) cache: Arc<RwLock<BindGroupCacheInternal>>,
}
impl BindGroupCache {
  pub(crate) fn clear(&self) {
    self.cache.write().clear();
  }

  pub fn create_dropper(&self, view_id: usize) -> BindGroupCacheInvalidation {
    BindGroupCacheInvalidation {
      view_id,
      cache: self.clone(),
    }
  }
}

pub struct BindGroupCacheInvalidation {
  pub(crate) view_id: usize,
  pub(crate) cache: BindGroupCache,
}

impl Drop for BindGroupCacheInvalidation {
  fn drop(&mut self) {
    self.cache.cache.write().notify_view_drop(self.view_id);
  }
}

#[derive(Clone, Default)]
pub struct BindGroupLayoutCache {
  /// keyed by the full layout entries to avoid hash collision
  pub cache: Arc<RwLock<FastHashMap<Vec<gpu::BindGroupLayoutEntry>, GPUBindGroupLayout>>>,
}

static BINDGROUP_LAYOUT_ID: AtomicU64 = AtomicU64::new(0);
/// the id is never reused, so it can be used as the exact identity of the layout
pub(crate) fn new_bindgroup_layout_id() -> u64 {
  BINDGROUP_LAYOUT_ID.fetch_add(1, Ordering::Relaxed)
}

impl BindGroupLayoutCache {
  pub(crate) fn clear(&self) {
    self.cache.write().clear();
  }
}
