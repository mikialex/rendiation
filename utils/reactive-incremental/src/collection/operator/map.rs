use crate::*;

pub struct ReactiveKVMap<T, F, K, V> {
  pub inner: T,
  pub map: F,
  pub phantom: PhantomData<(K, V)>,
}

impl<T, F, K, V, V2> ReactiveCollection<K, V2> for ReactiveKVMap<T, F, K, V>
where
  V: CValue,
  K: CKey,
  V2: CValue,
  F: Fn(V) -> V2 + Copy + Send + Sync + 'static,
  T: ReactiveCollection<K, V>,
{
  #[tracing::instrument(skip_all, name = "ReactiveKVMap")]
  fn poll_changes(&self, cx: &mut Context<'_>) -> PollCollectionChanges<K, V2> {
    self.inner.poll_changes(cx).map(|delta| {
      delta.map(|delta| {
        Box::new(MappedCollectionDelta {
          base: delta,
          mapper: self.map,
        }) as Box<dyn VirtualCollection<K, CollectionDelta<K, V2>>>
      })
    })
  }

  fn access(&self) -> PollCollectionCurrent<K, V2> {
    self.inner.access().map(|current| {
      Box::new(MappedCollection {
        base: current,
        mapper: self.map,
      })
    } as Box<dyn VirtualCollection<K, V2>>)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

#[derive(Clone)]
struct MappedCollection<'a, K, V, F> {
  base: Box<dyn VirtualCollection<K, V> + 'a>,
  mapper: F,
}

impl<'a, K, V, V2, F> VirtualCollection<K, V2> for MappedCollection<'a, K, V, F>
where
  K: CKey,
  V: CValue,
  V2: CValue,
  F: Fn(V) -> V2 + Copy + Send + Sync + 'static,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V2)> + '_> {
    Box::new(
      self
        .base
        .iter_key_value()
        .map(|(k, v)| (k, (self.mapper)(v))),
    )
  }

  fn access(&self, key: &K) -> Option<V2> {
    self.base.access(key).map(self.mapper)
  }
}

#[derive(Clone)]
struct MappedCollectionDelta<'a, K, V, F> {
  base: Box<dyn VirtualCollection<K, CollectionDelta<K, V>> + 'a>,
  mapper: F,
}

impl<'a, K, V, V2, F> VirtualCollection<K, CollectionDelta<K, V2>>
  for MappedCollectionDelta<'a, K, V, F>
where
  K: CKey,
  V: CValue,
  V2: CValue,
  F: Fn(V) -> V2 + Copy + Send + Sync + 'static,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, CollectionDelta<K, V2>)> + '_> {
    Box::new(
      self
        .base
        .iter_key_value()
        .map(|(k, delta)| (k, delta.map(self.mapper))),
    )
  }

  fn access(&self, key: &K) -> Option<CollectionDelta<K, V2>> {
    self.base.access(key).map(|delta| delta.map(self.mapper))
  }
}

/// compare to ReactiveKVMap, this execute immediately and not impose too many bounds on mapper
pub struct ReactiveKVExecuteMap<T, F, K, V, V2> {
  pub inner: T,
  pub map_creator: F,
  pub cache: dashmap::DashMap<K, V2, FastHasherBuilder>,
  pub phantom: PhantomData<(K, V, V2)>,
}

impl<T, F, K, V, V2, FF> ReactiveCollection<K, V2> for ReactiveKVExecuteMap<T, F, K, V, V2>
where
  V: CValue,
  K: CKey,
  F: Fn() -> FF + Send + Sync + 'static,
  FF: Fn(&K, V) -> V2 + Send + Sync + 'static,
  V2: CValue,
  T: ReactiveCollection<K, V>,
{
  #[tracing::instrument(skip_all, name = "ReactiveKVExecuteMap")]
  fn poll_changes(&self, cx: &mut Context<'_>) -> PollCollectionChanges<K, V2> {
    self.inner.poll_changes(cx).map(move |deltas| {
      deltas.map(|deltas| {
        let mapper = (self.map_creator)();
        let materialized = deltas.iter_key_value().collect::<Vec<_>>();
        let materialized: FastHashMap<K, CollectionDelta<K, V2>> = materialized
          .into_par_iter()
          .map(|(_, delta)| match delta {
            CollectionDelta::Delta(k, d, _p) => {
              let new_value = mapper(&k, d);
              let p = self.cache.insert(k.clone(), new_value.clone());
              (k.clone(), CollectionDelta::Delta(k, new_value, p))
            }
            CollectionDelta::Remove(k, _p) => {
              let (_, p) = self.cache.remove(&k).unwrap();
              (k.clone(), CollectionDelta::Remove(k, p))
            }
          })
          .collect();

        Box::new(Arc::new(materialized)) as Box<dyn VirtualCollection<K, CollectionDelta<K, V2>>>
      })
    })
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.cache.shrink_to_fit(),
    }
    self.inner.extra_request(request)
  }

  fn access(&self) -> PollCollectionCurrent<K, V2> {
    CPoll::Ready(Box::new(&self.cache as &dyn VirtualCollection<K, V2>))
  }
}
