use crate::*;

pub struct ControlledMemoryStreaming<K: CKey, V, URI> {
  load_control: LoadingThrottler<K, V, URI>,
  waker: Option<std::task::Waker>,
  full_resource_scope: FastHashMap<K, (URI, u64)>,
  retain_cost_limitation: u64,
  action: Option<SwapAction<K>>,
}

struct SwapAction<K> {
  remove_loaded: Vec<K>,
  new_load_request: Vec<K>,
}

impl<K: CKey, V, URI: Clone> ControlledMemoryStreaming<K, V, URI> {
  /// how iterator is computed is not our business. it's the scheduling decision
  pub fn compute_swap_action(
    &mut self,
    iter_from_most_important_to_least: impl Iterator<Item = K>,
  ) {
    let mut remove_loaded = Vec::new();
    let mut new_load_request = Vec::new();

    let mut budget = self.retain_cost_limitation;
    for k in iter_from_most_important_to_least {
      if let Some((_, cost)) = self.full_resource_scope.get(&k) {
        if budget >= *cost {
          budget -= cost;

          // add new loaded items
          if !self.load_control.is_in_request(&k) {
            new_load_request.push(k);
          }
        } else {
          // remove loaded items
          if self.load_control.is_loaded(&k) {
            remove_loaded.push(k.clone());
          }

          // remove loading items
          self.load_control.cancel_not_dispatched_load(&k);
        }
      } else {
        // the weight compute logic may computed in async/deferred way,
        // so they may have key that has been removed
      }
    }

    if let Some(waker) = self.waker.take() {
      waker.wake();
    }

    self.action = Some(SwapAction {
      remove_loaded,
      new_load_request,
    });
  }
}

impl<K: CKey, V, URI: Clone + Send + Sync + ProvideMemoryCostInfo> AbstractResourceStreaming
  for ControlledMemoryStreaming<K, V, URI>
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
    let cost = uri.memory_cost();
    self
      .full_resource_scope
      .insert(key.clone(), (uri.clone(), cost.retain));
  }

  fn notify_remove_resource(&mut self, key: &Self::Key) {
    self.full_resource_scope.remove(key);
  }

  fn reload_all_loaded(&mut self) {
    self.load_control.request_load_all_reloaded();
  }

  fn poll_loading(
    &mut self,
    cx: &mut Context,
    loader: &mut LoaderFunction<Self::UriLike, Self::Data>,
  ) -> LinearBatchChanges<Self::Key, Option<Self::Data>> {
    self.waker = Some(cx.waker().clone());

    let mut removes = Vec::new();

    if let Some(action) = self.action.take() {
      for k in &action.new_load_request {
        let uri = self.full_resource_scope.get(k).unwrap().0.clone();
        let cost = uri.memory_cost().loading_peak;
        self.load_control.request_load(k.clone(), uri, cost);
      }

      removes = action.remove_loaded;
    }

    let new_load_list = self.load_control.poll_loading(cx, loader);

    LinearBatchChanges {
      removed: removes,
      update_or_insert: new_load_list,
    }
  }
}
