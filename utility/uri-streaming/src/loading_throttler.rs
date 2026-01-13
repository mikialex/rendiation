use crate::*;

pub struct LoadingThrottler<K: CKey, V, URI> {
  bandwidth_limitation: u64,
  current_bandwidth_used: u64,
  waitings: FastHashMap<K, (URI, u64)>,
  futures: MappedFutures<K, LoadFuture<V>>,
  loading_uri: FastHashMap<K, (URI, bool, u64)>,
  loaded: FastHashMap<K, (URI, u64)>,
}

impl<K: CKey, V, URI: Clone> LoadingThrottler<K, V, URI> {
  pub fn new(bandwidth_limitation: u64) -> Self {
    Self {
      bandwidth_limitation,
      current_bandwidth_used: 0,
      waitings: FastHashMap::default(),
      futures: MappedFutures::new(),
      loading_uri: FastHashMap::default(),
      loaded: FastHashMap::default(),
    }
  }

  pub fn is_in_request(&self, k: &K) -> bool {
    self.waitings.contains_key(k) || self.loading_uri.contains_key(k) || self.loaded.contains_key(k)
  }

  pub fn is_loaded(&self, k: &K) -> bool {
    self.loaded.contains_key(k)
  }

  pub fn request_load(&mut self, k: K, uri: URI, cost: u64) {
    if self.loading_uri.contains_key(&k) {
      // should we assert this case?
      return;
    }
    if self.loaded.contains_key(&k) {
      // should we assert this case?
      return;
    }
    self.waitings.insert(k, (uri, cost));
  }

  pub fn cancel_not_dispatched_load(&mut self, k: &K) {
    self.waitings.remove(k);
    if let Some((_, should_load, _)) = self.loading_uri.get_mut(k) {
      // note, we not remove the loading set, because even if the future drop triggers cancellation,
      // the cancellation may not take effect immediately
      *should_load = false
    }
    self.loaded.remove(k);
  }

  pub fn request_load_all_reloaded(&mut self) {
    for (k, v) in &self.loaded {
      self.waitings.insert(k.clone(), v.clone());
    }
  }

  pub fn poll_loading(
    &mut self,
    cx: &mut Context,
    loader: &mut LoaderFunction<URI, V>,
  ) -> Vec<(K, Option<V>)> {
    let mut load_list = Vec::new();
    while let Poll::Ready(Some((key, loaded))) = self.futures.poll_next_unpin(cx) {
      let (uri, should_keep, cost) = self.loading_uri.remove(&key).unwrap();
      self.current_bandwidth_used -= cost;
      if should_keep {
        load_list.push((key.clone(), loaded));
        self.loaded.insert(key, (uri, cost));
      }
    }

    let mut to_loads = Vec::new();
    // todo, consider using a btree map to incrementally sort cost
    for (k, (_, cost)) in self.waitings.iter() {
      if self.current_bandwidth_used + cost <= self.bandwidth_limitation {
        self.current_bandwidth_used += cost;
        to_loads.push(k.clone());
      }
    }
    for to_load in to_loads {
      let (uri, cost) = self.waitings.remove(&to_load).unwrap();
      self
        .loading_uri
        .insert(to_load.clone(), (uri.clone(), true, cost));
      let load_future = loader(&uri);
      self.futures.insert(to_load, load_future);
    }

    load_list
  }
}
