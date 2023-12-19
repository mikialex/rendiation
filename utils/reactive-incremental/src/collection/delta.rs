use crate::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueChange<V> {
  // k, new_v, pre_v
  Delta(V, Option<V>),
  // k, pre_v
  Remove(V),
}

impl<V> ValueChange<V> {
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
}

pub trait ChangeMerge {
  /// return if exist after merge
  fn merge(&mut self, new: &Self) -> bool;
}

impl<V> ChangeMerge for ValueChange<V>
where
  V: Clone,
{
  fn merge(&mut self, new: &Self) -> bool {
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
