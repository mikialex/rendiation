use crate::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueChange<V> {
  // k, new_v, pre_v
  Delta(V, Option<V>),
  // k, pre_v
  Remove(V),
}

impl<V: CValue> ValueChange<V> {
  pub fn map<R>(self, mapper: impl Fn(V) -> R) -> ValueChange<R> {
    type Rt<R> = ValueChange<R>;
    match self {
      Self::Remove(pre) => {
        let mapped = mapper(pre);
        Rt::<R>::Remove(mapped)
      }
      Self::Delta(d, pre) => {
        let mapped = mapper(d);
        let mapped_pre = pre.map(mapper);
        Rt::<R>::Delta(mapped, mapped_pre)
      }
    }
  }

  pub fn new_value(&self) -> Option<&V> {
    match self {
      Self::Delta(v, _) => Some(v),
      Self::Remove(_) => None,
    }
  }

  pub fn old_value(&self) -> Option<&V> {
    match self {
      Self::Delta(_, Some(v)) => Some(v),
      Self::Remove(v) => Some(v),
      _ => None,
    }
  }
  pub fn is_removed(&self) -> bool {
    match self {
      Self::Remove(_) => true,
      Self::Delta(_, _) => false,
    }
  }
  pub fn is_new_insert(&self) -> bool {
    matches!(self, Self::Delta(_, None))
  }

  pub fn is_redundant(&self) -> bool
  where
    V: PartialEq,
  {
    match self {
      ValueChange::Delta(v, pv) => {
        if let Some(pv) = pv {
          v == pv
        } else {
          false
        }
      }
      ValueChange::Remove(_) => false,
    }
  }

  /// return if exist after merge
  pub fn merge(&mut self, new: &Self) -> bool {
    use ValueChange::*;
    *self = match (self.clone(), new.clone()) {
      (Delta(_d1, p1), Delta(d2, _p2)) => {
        // we should check d1 = d2
        Delta(d2, p1)
      }
      (Delta(_d1, p1), Remove(_p2)) => {
        // we should check d1 = d2
        if let Some(p1) = p1 {
          Remove(p1)
        } else {
          return false;
        }
      }
      (Remove(p), Delta(d1, p2)) => {
        assert!(p2.is_none());
        Delta(d1, Some(p))
      }
      (Remove(_), Remove(_)) => {
        unreachable!("same key with double remove is invalid")
      }
    };

    true
  }
}

pub(crate) fn merge_into_hashmap<K: CKey, V: CValue>(
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

pub trait MutableCollection<K, V> {
  fn set_value(&mut self, k: K, v: V) -> Option<V>;
  fn remove(&mut self, k: K) -> Option<V>;
}

impl<K: CKey, V: CValue> MutableCollection<K, V> for FastHashMap<K, V> {
  fn set_value(&mut self, k: K, v: V) -> Option<V> {
    self.insert(k, v)
  }

  fn remove(&mut self, k: K) -> Option<V> {
    self.remove(&k)
  }
}
impl<'a, K: CKey, V: CValue> MutableCollection<K, V> for &'a mut FastHashMap<K, V> {
  fn set_value(&mut self, k: K, v: V) -> Option<V> {
    self.insert(k, v)
  }

  fn remove(&mut self, k: K) -> Option<V> {
    (*self).remove(&k)
  }
}

pub struct CollectionMutationCollector<D, T> {
  pub delta: D,
  pub target: T,
}

impl<K, V, D, T> MutableCollection<K, V> for CollectionMutationCollector<D, T>
where
  D: MutableCollection<K, ValueChange<V>>,
  T: MutableCollection<K, V>,
  K: CKey,
  V: CValue,
{
  fn set_value(&mut self, k: K, v: V) -> Option<V> {
    let previous = self.target.set_value(k.clone(), v.clone());
    let mut previous_delta = self.delta.remove(k.clone());
    let new_delta = ValueChange::Delta(v, previous.clone());

    if let Some(previous_delta) = &mut previous_delta {
      if previous_delta.merge(&new_delta) {
        self.delta.set_value(k, previous_delta.clone());
      }
    }

    previous
  }

  fn remove(&mut self, k: K) -> Option<V> {
    let previous = self.target.remove(k.clone());

    if let Some(previous) = previous.clone() {
      let mut previous_delta = self.delta.remove(k.clone());
      let new_delta = ValueChange::Remove(previous);

      if let Some(previous_delta) = &mut previous_delta {
        if previous_delta.merge(&new_delta) {
          self.delta.set_value(k, previous_delta.clone());
        }
      }
    }

    previous
  }
}
