use crate::*;

pub fn make_checker<V, V2>(
  checker: impl Fn(V) -> Option<V2> + Copy + Send + Sync + 'static,
) -> impl Fn(ValueChange<V>) -> Option<ValueChange<V2>> + Copy + Send + Sync + 'static {
  move |delta| {
    match delta {
      ValueChange::Delta(v, pre_v) => {
        let new_map = checker(v);
        let pre_map = pre_v.and_then(checker);
        match (new_map, pre_map) {
          (Some(v), Some(pre_v)) => ValueChange::Delta(v, Some(pre_v)),
          (Some(v), None) => ValueChange::Delta(v, None),
          (None, Some(pre_v)) => ValueChange::Remove(pre_v),
          (None, None) => return None,
        }
        .into()
      }
      // the Remove variant maybe called many times for given k
      ValueChange::Remove(pre_v) => {
        let pre_map = checker(pre_v);
        match pre_map {
          Some(pre) => ValueChange::Remove(pre).into(),
          None => None,
        }
      }
    }
  }
}

pub struct ReactiveKVFilter<T, F, K, V> {
  pub inner: T,
  pub checker: F,
  pub k: PhantomData<(K, V)>,
}

impl<T, F, K, V, V2> ReactiveCollection<K, V2> for ReactiveKVFilter<T, F, K, V>
where
  F: Fn(V) -> Option<V2> + Copy + Send + Sync + 'static,
  T: ReactiveCollection<K, V> + Sync,
  K: CKey,
  V: CValue,
  V2: CValue,
{
  #[tracing::instrument(skip_all, name = "ReactiveKVFilter")]
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V2> {
    self.inner.poll_changes(cx).map(|delta| {
      delta.map(|delta| {
        Box::new(ValueChangeFilter {
          base: delta,
          mapper: self.checker,
        }) as Box<dyn VirtualCollection<K, ValueChange<V2>>>
      })
    })
  }

  fn access(&self) -> PollCollectionCurrent<K, V2> {
    self.inner.access().map(|current| {
      Box::new(CollectionFilter {
        base: current,
        mapper: self.checker,
      })
    } as Box<dyn VirtualCollection<K, V2>>)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

#[derive(Clone)]
struct CollectionFilter<'a, K, V, F> {
  base: Box<dyn VirtualCollection<K, V> + 'a>,
  mapper: F,
}

impl<'a, K, V, F, V2> VirtualCollection<K, V2> for CollectionFilter<'a, K, V, F>
where
  F: Fn(V) -> Option<V2> + Sync + Send + Copy + 'static,
  K: CKey,
  V: CValue,
  V2: CValue,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, V2)> + '_> {
    Box::new(
      self
        .base
        .iter_key_value()
        .filter_map(|(k, v)| (self.mapper)(v).map(|v| (k, v))),
    )
  }

  fn access(&self, key: &K) -> Option<V2> {
    let base = self.base.access(key)?;
    (self.mapper)(base)
  }
}

#[derive(Clone)]
struct ValueChangeFilter<'a, K, V, F> {
  base: Box<dyn VirtualCollection<K, ValueChange<V>> + 'a>,
  mapper: F,
}

impl<'a, K, V, F, V2> VirtualCollection<K, ValueChange<V2>> for ValueChangeFilter<'a, K, V, F>
where
  F: Fn(V) -> Option<V2> + Sync + Send + Copy + 'static,
  K: CKey,
  V: CValue,
  V2: CValue,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, ValueChange<V2>)> + '_> {
    let checker = make_checker(self.mapper);
    Box::new(
      self
        .base
        .iter_key_value()
        .filter_map(move |(k, v)| checker(v).map(|v| (k, v))),
    )
  }

  fn access(&self, key: &K) -> Option<ValueChange<V2>> {
    let checker = make_checker(self.mapper);
    let base = self.base.access(key)?;
    checker(base)
  }
}
