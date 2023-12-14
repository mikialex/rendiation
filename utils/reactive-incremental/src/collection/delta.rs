use crate::*;

#[derive(Debug, Clone, Copy)]
pub enum CollectionDelta<K, V> {
  // k, new_v, pre_v
  Delta(K, V, Option<V>),
  // k, pre_v
  Remove(K, V),
}

impl<K, V> CollectionDelta<K, V> {
  pub fn map<R>(self, mapper: impl Fn(V) -> R) -> CollectionDelta<K, R> {
    type Rt<K, R> = CollectionDelta<K, R>;
    match self {
      Self::Remove(k, pre) => {
        let mapped = mapper(pre);
        Rt::<K, R>::Remove(k, mapped)
      }
      Self::Delta(k, d, pre) => {
        let mapped = mapper(d);
        let mapped_pre = pre.map(mapper);
        Rt::<K, R>::Delta(k, mapped, mapped_pre)
      }
    }
  }

  pub fn key(&self) -> &K {
    match self {
      Self::Remove(k, _) => k,
      Self::Delta(k, _, _) => k,
    }
  }

  pub fn new_value(&self) -> Option<&V> {
    match self {
      Self::Delta(_, v, _) => Some(v),
      Self::Remove(_, _) => None,
    }
  }

  pub fn old_value(&self) -> Option<&V> {
    match self {
      Self::Delta(_, _, Some(v)) => Some(v),
      Self::Remove(_, v) => Some(v),
      _ => None,
    }
  }
  pub fn is_removed(&self) -> bool {
    match self {
      Self::Remove(_, _) => true,
      Self::Delta(_, _, _) => false,
    }
  }
}

pub trait ChangeMerge {
  /// return if exist after merge
  fn merge(&mut self, new: &Self) -> bool;
}

impl<K, V> ChangeMerge for CollectionDelta<K, V>
where
  K: PartialEq + Clone,
  V: Clone,
{
  fn merge(&mut self, new: &Self) -> bool {
    use CollectionDelta::*;
    if self.key() != new.key() {
      panic!("only same key change could be merge");
    }
    *self = match (self.clone(), new.clone()) {
      (Delta(k, _d1, p1), Delta(_, d2, _p2)) => {
        // we should check d1 = d2
        Delta(k, d2, p1)
      }
      (Delta(k, _d1, p1), Remove(_, _p2)) => {
        // we should check d1 = d2
        if let Some(p1) = p1 {
          Remove(k, p1)
        } else {
          return false;
        }
      }
      (Remove(k, p), Delta(_, d1, p2)) => {
        assert!(p2.is_none());
        Delta(k, d1, Some(p))
      }
      (Remove(_, _), Remove(_, _)) => {
        unreachable!("same key with double remove is invalid")
      }
    };

    true
  }
}

impl<K, V> ChangeMerge for FastHashMap<K, V>
where
  K: Eq + std::hash::Hash + Clone,
  V: Clone + ChangeMerge,
{
  fn merge(&mut self, new: &Self) -> bool {
    new.iter().for_each(|(k, d)| {
      let key = k.clone();
      if let Some(current) = self.get_mut(&key) {
        if !current.merge(d) {
          self.remove(&key);
        }
      } else {
        self.insert(key, d.clone());
      }
    });
    !self.is_empty()
  }
}
