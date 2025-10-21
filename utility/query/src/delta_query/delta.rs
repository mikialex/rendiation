use crate::*;

#[derive(Serialize, Deserialize)]
#[derive(Clone, Copy, PartialEq, Hash, Eq)]
pub enum ValueChange<V> {
  // k, new_v, pre_v
  Delta(V, Option<V>),
  // k, pre_v
  Remove(V),
}

impl<V: std::fmt::Debug> std::fmt::Debug for ValueChange<V> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Delta(arg0, arg1) => {
        if let Some(arg1) = arg1 {
          write!(f, "change(from {:?} to {:?})", arg1, arg0)
        } else {
          write!(f, "new({:?})", arg0)
        }
      }
      Self::Remove(arg0) => write!(f, "removed({:?})", arg0),
    }
  }
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

  pub fn into_new_value(self) -> Option<V> {
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
  pub fn merge(&mut self, new: &Self) -> bool
  where
    V: Clone,
  {
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

pub fn make_checker<V, V2>(
  checker: impl Fn(V) -> Option<V2> + Clone + Send + Sync + 'static,
) -> impl Fn(ValueChange<V>) -> Option<ValueChange<V2>> + Clone + Send + Sync + 'static {
  move |delta| {
    match delta {
      ValueChange::Delta(v, pre_v) => {
        let new_map = checker(v);
        let pre_map = pre_v.and_then(checker.clone());
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

pub fn merge_change<K: CKey, T: CValue>(
  mutations: &mut FastHashMap<K, ValueChange<T>>,
  (idx, change): (K, ValueChange<T>),
) {
  if let Some(old_change) = mutations.get_mut(&idx) {
    if !old_change.merge(&change) {
      mutations.remove(&idx);
    }
  } else {
    mutations.insert(idx, change);
  }
}

pub fn integrate_change<K: CKey, T: CValue>(
  states: &mut FastHashMap<K, T>,
  (idx, change): (K, ValueChange<T>),
) {
  match change {
    ValueChange::Delta(new, _) => {
      states.insert(idx, new);
    }
    ValueChange::Remove(_) => {
      states.remove(&idx);
    }
  }
}

pub fn validate_delta<K: CKey, V: CValue>(
  state: &mut FastHashMap<K, V>,
  log_change: bool,
  label: &str,
  d: &impl Query<Key = K, Value = ValueChange<V>>,
) {
  let changes = d.materialize();

  if !changes.is_empty() && log_change {
    println!("change details for <{}>:", label);
  }
  for (k, change) in changes.iter() {
    if log_change {
      println!("{:?}: {:?}", k, change);
    }
    match change {
      ValueChange::Delta(n, p) => {
        if let Some(removed) = state.remove(k) {
          let p = p.as_ref();

          if p.is_none() {
            panic!("previous value should exist, {}", label);
          }

          assert_eq!(&removed, p.unwrap(), "{}", label);
        } else {
          assert!(p.is_none());
        }
        state.insert(k.clone(), n.clone());
      }
      ValueChange::Remove(p) => {
        let removed = state.remove(k);

        if removed.is_none() {
          panic!("remove none exist value, {}", label);
        }

        assert_eq!(&removed.unwrap(), p, "{}", label);
      }
    }
  }
}
