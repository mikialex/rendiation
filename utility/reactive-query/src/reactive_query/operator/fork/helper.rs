use crate::*;

pub fn finalize_buffered_changes<K: CKey, V: CValue>(
  mut changes: Vec<Arc<FastHashMap<K, ValueChange<V>>>>,
) -> BoxedDynQuery<K, ValueChange<V>> {
  if changes.is_empty() {
    return Box::new(EmptyQuery::default());
  }

  if changes.len() == 1 {
    let first = changes.pop().unwrap();
    if first.is_empty() {
      return Box::new(EmptyQuery::default());
    } else {
      return Box::new(first);
    }
  }

  let mut target = FastHashMap::default();

  for c in changes {
    merge_into_hashmap(&mut target, c.iter().map(|(k, v)| (k.clone(), v.clone())));
  }

  if target.is_empty() {
    Box::new(EmptyQuery::default())
  } else {
    Box::new(target)
  }
}

fn merge_into_hashmap<K: CKey, V: CValue>(
  map: &mut FastHashMap<K, ValueChange<V>>,
  iter: impl Iterator<Item = (K, ValueChange<V>)>,
) {
  iter.for_each(|(k, v)| {
    if let Some(current) = map.get_mut(&k) {
      if !current.merge(&v) {
        map.remove(&k);
      }
    } else {
      map.insert(k, v.clone());
    }
  })
}

#[derive(Clone)]
pub struct ForkedView<T> {
  pub inner: Arc<T>,
}

impl<T: Query> Query for ForkedView<T> {
  type Key = T::Key;
  type Value = T::Value;
  fn iter_key_value(&self) -> impl Iterator<Item = (T::Key, T::Value)> + '_ {
    self.inner.iter_key_value()
  }

  fn access(&self, key: &T::Key) -> Option<T::Value> {
    self.inner.access(key)
  }
}
impl<T: MultiQuery> MultiQuery for ForkedView<T> {
  type Key = T::Key;
  type Value = T::Value;
  fn iter_keys(&self) -> impl Iterator<Item = T::Key> + '_ {
    self.inner.iter_keys()
  }

  fn access_multi(&self, key: &T::Key) -> Option<impl Iterator<Item = T::Value> + '_> {
    self.inner.access_multi(key)
  }
}
