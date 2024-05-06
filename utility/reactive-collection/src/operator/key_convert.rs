use crate::*;

pub struct ReactiveKeyConvert<F1, F2, T, K, V> {
  pub f1: F1,
  pub f2: F2,
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<F1, F2, T, K, K2, V> ReactiveCollection<K2, V> for ReactiveKeyConvert<F1, F2, T, K, V>
where
  K: CKey,
  K2: CKey,
  V: CValue,
  F1: Fn(K) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> K + Copy + Send + Sync + 'static,
  T: ReactiveCollection<K, V>,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K2, V> {
    self.inner.poll_changes(cx).map(|delta| {
      Box::new(KeyConvertValueChange {
        base: delta,
        f1: self.f1,
        f2: self.f2,
      }) as Box<dyn VirtualCollection<K2, ValueChange<V>>>
    })
  }

  fn access(&self) -> PollCollectionCurrent<K2, V> {
    Box::new(KeyConvertCollection {
      base: self.inner.access(),
      f1: self.f1,
      f2: self.f2,
    })
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

#[derive(Clone)]
struct KeyConvertValueChange<'a, K, V, F1, F2> {
  base: Box<dyn VirtualCollection<K, ValueChange<V>> + 'a>,
  pub f1: F1,
  pub f2: F2,
}

impl<'a, K, K2, V, F1, F2> VirtualCollection<K2, ValueChange<V>>
  for KeyConvertValueChange<'a, K, V, F1, F2>
where
  K: CKey,
  K2: CKey,
  V: CValue,
  F1: Fn(K) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> K + Copy + Send + Sync + 'static,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K2, ValueChange<V>)> + '_> {
    Box::new(self.base.iter_key_value().map(|(k, v)| {
      let k = (self.f1)(k);
      (k, v)
    }))
  }

  fn access(&self, key: &K2) -> Option<ValueChange<V>> {
    self.base.access(&(self.f2)(key.clone()))
  }
}

#[derive(Clone)]
struct KeyConvertCollection<'a, K, V, F1, F2> {
  base: Box<dyn VirtualCollection<K, V> + 'a>,
  pub f1: F1,
  pub f2: F2,
}

impl<'a, K, K2, V, F1, F2> VirtualCollection<K2, V> for KeyConvertCollection<'a, K, V, F1, F2>
where
  K: CKey,
  K2: CKey,
  V: CValue,
  F1: Fn(K) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> K + Copy + Send + Sync + 'static,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K2, V)> + '_> {
    Box::new(self.base.iter_key_value().map(|(k, v)| {
      let k = (self.f1)(k);
      (k, v)
    }))
  }

  fn access(&self, key: &K2) -> Option<V> {
    self.base.access(&(self.f2)(key.clone()))
  }
}
