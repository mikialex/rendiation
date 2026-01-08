use crate::*;

/// the basic implementation is load what your request to load
pub struct NoScheduleScheduler<K: CKey, V, URI> {
  pub futures: MappedFutures<K, LoadFuture<V>>,
  pub loading_uri: FastHashMap<K, URI>,
  pub loaded: FastHashMap<K, URI>,
  pub request_reload: bool,
}

impl<K: CKey, V, URI> Default for NoScheduleScheduler<K, V, URI> {
  fn default() -> Self {
    Self {
      futures: MappedFutures::new(),
      loaded: FastHashMap::default(),
      request_reload: false,
      loading_uri: FastHashMap::default(),
    }
  }
}

impl<K: CKey, V, URI: Clone + Send + Sync> AbstractResourceScheduler
  for NoScheduleScheduler<K, V, URI>
{
  type Data = V;
  type Key = K;
  type UriLike = URI;

  fn notify_use_resource(
    &mut self,
    key: &Self::Key,
    uri: &URI,
    loader: &mut LoaderFunction<URI, V>,
  ) {
    let future = loader(uri);
    self.futures.replace(key.clone(), future);
    self.loading_uri.insert(key.clone(), uri.clone());
  }

  fn notify_remove_resource(&mut self, key: &Self::Key) {
    self.futures.remove(key);
    self.loaded.remove(key);
  }

  fn reload_all_loaded(&mut self) {
    self.request_reload = true;
  }

  fn poll_schedule(
    &mut self,
    cx: &mut Context,
    loader: &mut LoaderFunction<URI, V>,
  ) -> LinearBatchChanges<Self::Key, Option<Self::Data>> {
    if self.request_reload {
      self.request_reload = false;
      for (key, uri) in &self.loaded {
        let future = loader(uri);
        self.futures.replace(key.clone(), future);
        self.loading_uri.insert(key.clone(), uri.clone());
      }
    }

    let mut load_list = Vec::new();
    while let Poll::Ready(Some((key, loaded))) = self.futures.poll_next_unpin(cx) {
      load_list.push((key.clone(), loaded));
      let uri = self.loading_uri.remove(&key).unwrap();
      self.loaded.insert(key, uri);
    }

    LinearBatchChanges {
      removed: Vec::new(), // this can be empty, because it will removed by caller anyway
      update_or_insert: load_list,
    }
  }
}
