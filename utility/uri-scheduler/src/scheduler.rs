use crate::*;

pub struct ControlledMemoryScheduler<K: CKey, V, URI> {
  pub futures: MappedFutures<K, LoadFuture<V>>,
  pub loading_uri: FastHashMap<K, (URI, bool)>,
  pub loaded: FastHashMap<K, URI>,
  pub request_reload: bool,
  waker: Option<std::task::Waker>,
  full_resource_scope: FastHashMap<K, (URI, ResourceMemoryCost)>,
  retain_cost_limitation: u64,
  schedule_action: Option<ScheduleAction<K>>,
}

struct ScheduleAction<K> {
  remove_loaded: Vec<K>,
  new_load_request: Vec<K>,
}

impl<K: CKey, V, URI> ControlledMemoryScheduler<K, V, URI> {
  /// how iterator is computed is not our business.
  pub fn do_schedule(&mut self, iter_from_most_important_to_least: impl Iterator<Item = K>) {
    let mut remove_loaded = Vec::new();
    let mut new_load_request = Vec::new();

    let mut budget = self.retain_cost_limitation;
    // let mut fit_count = 0;
    for k in iter_from_most_important_to_least {
      if let Some((_, cost)) = self.full_resource_scope.get(&k) {
        if budget >= cost.retain {
          //   fit_count += 1;
          budget -= cost.retain;

          // add new loaded items
          if !self.loaded.contains_key(&k) && !self.loading_uri.contains_key(&k) {
            new_load_request.push(k);
          }
        } else {
          // remove loading items
          if let Some((_, should_load)) = self.loading_uri.get_mut(&k) {
            // note, we not remove the loading set, because even if the future drop triggers cancellation,
            // the cancellation may not take effect immediately
            *should_load = false;
          }

          // remove loaded items
          if self.loaded.contains_key(&k) {
            remove_loaded.push(k);
          }
        }
      } else {
        // the weight compute logic may computed in async/deferred way,
        // so they may have key that has been removed
      }
    }

    if let Some(waker) = self.waker.take() {
      waker.wake();
    }

    self.schedule_action = Some(ScheduleAction {
      remove_loaded,
      new_load_request,
    });
  }
}

impl<K: CKey, V, URI: Clone + Send + Sync + ProvideMemoryCostInfo> AbstractResourceScheduler
  for ControlledMemoryScheduler<K, V, URI>
{
  type Data = V;
  type Key = K;
  type UriLike = URI;

  fn notify_use_resource(
    &mut self,
    key: &Self::Key,
    uri: &Self::UriLike,
    _loader: &mut LoaderFunction<Self::UriLike, Self::Data>,
  ) {
    self
      .full_resource_scope
      .insert(key.clone(), (uri.clone(), uri.memory_cost()));
  }

  fn notify_remove_resource(&mut self, key: &Self::Key) {
    self.full_resource_scope.remove(key);
  }

  fn reload_all_loaded(&mut self) {
    self.request_reload = true;
  }

  fn poll_schedule(
    &mut self,
    cx: &mut Context,
    loader: &mut LoaderFunction<Self::UriLike, Self::Data>,
  ) -> LinearBatchChanges<Self::Key, Option<Self::Data>> {
    self.waker = Some(cx.waker().clone());

    let mut removes = Vec::new();

    if let Some(action) = self.schedule_action.take() {
      for k in &action.remove_loaded {
        self.loaded.remove(k);
      }

      for k in &action.new_load_request {
        let uri = self.full_resource_scope.get(k).unwrap().0.clone();
        let load_future = loader(&uri);
        self.loading_uri.insert(k.clone(), (uri, true));
        self.futures.insert(k.clone(), load_future);
      }

      removes = action.remove_loaded;
    }

    let mut load_list = Vec::new();
    while let Poll::Ready(Some((key, loaded))) = self.futures.poll_next_unpin(cx) {
      let (uri, should_keep) = self.loading_uri.remove(&key).unwrap();
      if should_keep {
        load_list.push((key.clone(), loaded));
        self.loaded.insert(key, uri);
      }
    }

    LinearBatchChanges {
      removed: removes,
      update_or_insert: load_list,
    }
  }
}
