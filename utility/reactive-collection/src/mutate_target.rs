use crate::*;

pub trait CollectionLikeMutateTarget<K, V: CValue> {
  fn get_current(&self, k: K) -> Option<&V>;
  /// this method is useful if you want to modify part of V,
  /// we use the CPS style here to make sure callee could do sth after caller mutation.
  fn mutate(&mut self, k: K, mutator: &dyn Fn(&mut V));
  fn remove(&mut self, k: K) -> Option<V>;

  /// return previous value if v exist before
  fn set_value(&mut self, k: K, v: V) -> Option<V>;
}

impl<'a, K: CKey, V: CValue, T: CollectionLikeMutateTarget<K, V>> CollectionLikeMutateTarget<K, V>
  for &'a mut T
{
  fn get_current(&self, k: K) -> Option<&V> {
    (**self).get_current(k)
  }
  fn mutate(&mut self, k: K, mutator: &dyn Fn(&mut V)) {
    (**self).mutate(k, mutator)
  }
  fn remove(&mut self, k: K) -> Option<V> {
    (**self).remove(k)
  }
  fn set_value(&mut self, k: K, v: V) -> Option<V> {
    (**self).set_value(k, v)
  }
}

impl<K: CKey, V: CValue> CollectionLikeMutateTarget<K, V> for FastHashMap<K, V> {
  fn set_value(&mut self, k: K, v: V) -> Option<V> {
    self.insert(k, v)
  }

  fn remove(&mut self, k: K) -> Option<V> {
    self.remove(&k)
  }

  fn get_current(&self, k: K) -> Option<&V> {
    self.get(&k)
  }

  fn mutate(&mut self, k: K, mutator: &dyn Fn(&mut V)) {
    if let Some(r) = self.get_mut(&k) {
      mutator(r)
    }
  }
}
impl<T: CValue> CollectionLikeMutateTarget<u32, T> for IndexKeptVec<T> {
  fn set_value(&mut self, k: u32, v: T) -> Option<T> {
    let previous = self.try_get(k).cloned();
    self.insert(v, k);
    previous
  }

  fn remove(&mut self, k: u32) -> Option<T> {
    IndexKeptVec::remove(self, k)
  }

  fn get_current(&self, k: u32) -> Option<&T> {
    self.try_get(k)
  }

  fn mutate(&mut self, k: u32, mutator: &dyn Fn(&mut T)) {
    if let Some(r) = self.try_get_mut(k) {
      mutator(r)
    }
  }
}

pub struct CollectionMutationCollector<D, T> {
  pub delta: D,
  pub target: T,
}

impl<K, V, D, T> CollectionLikeMutateTarget<K, V> for CollectionMutationCollector<D, T>
where
  D: CollectionLikeMutateTarget<K, ValueChange<V>>,
  T: CollectionLikeMutateTarget<K, V>,
  K: CKey,
  V: CValue,
{
  fn get_current(&self, k: K) -> Option<&V> {
    self.target.get_current(k)
  }

  fn mutate(&mut self, k: K, mutator: &dyn Fn(&mut V)) {
    let previous = self.target.get_current(k.clone()).unwrap().clone();
    self.target.mutate(k.clone(), mutator);
    let after = self.target.get_current(k.clone()).unwrap().clone();
    let new_delta = ValueChange::Delta(after, Some(previous));

    let mut previous_delta = self.delta.remove(k.clone());
    if let Some(previous_delta) = &mut previous_delta {
      if previous_delta.merge(&new_delta) {
        self.delta.set_value(k, previous_delta.clone());
      }
    } else {
      self.delta.set_value(k, new_delta);
    }
  }

  fn set_value(&mut self, k: K, v: V) -> Option<V> {
    let previous = self.target.set_value(k.clone(), v.clone());
    let new_delta = ValueChange::Delta(v, previous.clone());

    let mut previous_delta = self.delta.remove(k.clone());
    if let Some(previous_delta) = &mut previous_delta {
      if previous_delta.merge(&new_delta) {
        self.delta.set_value(k, previous_delta.clone());
      }
    } else {
      self.delta.set_value(k, new_delta);
    }

    previous
  }

  fn remove(&mut self, k: K) -> Option<V> {
    let previous = self.target.remove(k.clone());

    if let Some(previous) = previous.clone() {
      let new_delta = ValueChange::Remove(previous);
      let mut previous_delta = self.delta.remove(k.clone());

      if let Some(previous_delta) = &mut previous_delta {
        if previous_delta.merge(&new_delta) {
          self.delta.set_value(k, previous_delta.clone());
        }
      }
    }

    previous
  }
}
