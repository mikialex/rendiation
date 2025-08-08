use crate::*;

type BindgroupHashKey = u64;
type ViewId = usize;

/// Key point of the cache control logic:
/// - bindgroup and resource_view is many-to-many relation.
/// - resource_view drop triggers all related bindgroup drop.
/// - bindgroup itself never been triggered drop.
// todo, merge per item allocation into single one.
#[derive(Default)]
pub struct BindGroupCacheInternal {
  bindgroups: FastHashMap<BindgroupHashKey, (gpu::BindGroup, Counted<gpu::BindGroup>, Vec<ViewId>)>,
  // todo, fix potential O(n) remove
  resource_views_bindgroups: FastHashMap<ViewId, Vec<BindgroupHashKey>>,
}

impl BindGroupCacheInternal {
  pub fn cached_binding_count(&self) -> usize {
    self.bindgroups.len()
  }

  pub fn clear(&mut self) {
    self.bindgroups.clear();
    self.resource_views_bindgroups.clear();
  }

  #[allow(clippy::manual_inspect)]
  pub fn get_or_create(
    &mut self,
    key: BindgroupHashKey,
    create: impl FnOnce() -> gpu::BindGroup,
    iter_view_id: impl Iterator<Item = ViewId>,
  ) -> &gpu::BindGroup {
    let (bindgroup, _, _) = self.bindgroups.entry(key).or_insert_with(|| {
      let bindgroup = create();
      let list = iter_view_id
        .map(|view_id| {
          self
            .resource_views_bindgroups
            .entry(view_id)
            .or_default()
            .push(key);

          view_id
        })
        .collect();
      (bindgroup, Default::default(), list)
    });
    bindgroup
  }

  pub fn notify_view_drop(&mut self, view_id: ViewId) {
    if let Some(all_referenced_bindings) = self.resource_views_bindgroups.remove(&view_id) {
      for binding in all_referenced_bindings {
        if let Some((_, _, binding_referenced_views)) = self.bindgroups.remove(&binding) {
          for view_id in binding_referenced_views {
            if let Some(bindings) = self.resource_views_bindgroups.get_mut(&view_id) {
              bindings
                .iter()
                .position(|v| *v == binding)
                .map(|v| bindings.swap_remove(v));
              if bindings.is_empty() {
                self.resource_views_bindgroups.remove(&view_id);
              }
            }
          }
        } // none is possible because we allow cache clear
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
  pub cache: Arc<RwLock<FastHashMap<u64, GPUBindGroupLayout>>>,
}

impl BindGroupLayoutCache {
  pub(crate) fn clear(&self) {
    self.cache.write().clear();
  }
}
