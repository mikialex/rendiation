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
    V: Clone + PartialEq,
  {
    use ValueChange::*;
    *self = match (self.clone(), new.clone()) {
      (Delta(_d1, p1), Delta(d2, _p2)) => {
        // We intentionally do NOT validate that _d1 == _p2 (the previous
        // delta's new value equals the incoming delta's old value).  For
        // floats, PartialEq treats NaN != NaN, yet two NaN values in a
        // change sequence are logically consistent (no real change
        // occurred).  There is no generic way to express "equal or both
        // NaN" without a custom trait that would burden all non-float
        // uses of ValueChange.
        if let Some(p1) = &p1 {
          if p1 == &d2 {
            return false;
          }
        }
        Delta(d2, p1)
      }
      (Delta(_d1, p1), Remove(_p2)) => {
        // See the Delta+Delta branch for why we skip _d1 == _p2 validation.
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

#[test]
fn test_value_change_map() {
  let c = ValueChange::Delta(10i32, Some(5));
  let mapped = c.map(|v| v * 2);
  assert_eq!(mapped, ValueChange::Delta(20, Some(10)));

  let r = ValueChange::<i32>::Remove(5);
  let mapped = r.map(|v| v * 2);
  assert_eq!(mapped, ValueChange::Remove(10));
}

#[test]
fn test_value_change_merge_delta_delta() {
  // consecutive deltas: keeps old previous
  let mut c = ValueChange::Delta(10i32, Some(5));
  let exists = c.merge(&ValueChange::Delta(15, Some(10)));
  assert!(exists);
  assert_eq!(c, ValueChange::Delta(15, Some(5)));

  // redundant: same value cancels out
  let mut c = ValueChange::Delta(10i32, Some(10));
  let exists = c.merge(&ValueChange::Delta(10, Some(10)));
  assert!(!exists);
}

#[test]
fn test_value_change_merge_delta_remove() {
  // delta then remove → stays remove with original previous
  let mut c = ValueChange::Delta(10i32, Some(5));
  let exists = c.merge(&ValueChange::Remove(10));
  assert!(exists);
  assert_eq!(c, ValueChange::Remove(5));

  // new insert then remove → cancels out entirely
  let mut c = ValueChange::Delta(10i32, None);
  let exists = c.merge(&ValueChange::Remove(10));
  assert!(!exists);
}

#[test]
fn test_value_change_merge_remove_delta() {
  // remove then re-insert (different value) → delta
  let mut c = ValueChange::<i32>::Remove(5);
  let exists = c.merge(&ValueChange::Delta(10, None));
  assert!(exists);
  assert_eq!(c, ValueChange::Delta(10, Some(5)));
}

#[test]
fn test_value_change_accessors() {
  let insert = ValueChange::Delta(10i32, None);
  assert!(insert.is_new_insert());
  assert!(!insert.is_removed());
  assert_eq!(insert.new_value(), Some(&10));
  assert_eq!(insert.old_value(), None);
  assert!(!insert.is_redundant());

  let update = ValueChange::Delta(20i32, Some(10));
  assert!(!update.is_new_insert());
  assert_eq!(update.old_value(), Some(&10));
  assert!(!update.is_redundant());

  let same = ValueChange::Delta(10i32, Some(10));
  assert!(same.is_redundant());
  assert!(same.is_redundant());

  let remove = ValueChange::Remove(5i32);
  assert!(remove.is_removed());
  assert_eq!(remove.old_value(), Some(&5));
  assert_eq!(remove.new_value(), None);
}

#[test]
fn test_make_checker() {
  let checker = make_checker(|v: i32| if v > 5 { Some(v * 2) } else { None });

  // delta: both new and old pass
  assert_eq!(
    checker(ValueChange::Delta(10, Some(8))),
    Some(ValueChange::Delta(20, Some(16)))
  );

  // delta: only new passes → Delta with None previous
  assert_eq!(
    checker(ValueChange::Delta(10, Some(3))),
    Some(ValueChange::Delta(20, None))
  );

  // delta: only old passes → Remove
  assert_eq!(
    checker(ValueChange::Delta(3, Some(8))),
    Some(ValueChange::Remove(16))
  );

  // delta: neither passes
  assert_eq!(checker(ValueChange::Delta(3, Some(2))), None);

  // remove: old passes → Remove
  assert_eq!(checker(ValueChange::Remove(8)), Some(ValueChange::Remove(16)));

  // remove: old doesn't pass
  assert_eq!(checker(ValueChange::Remove(3)), None);
}

#[test]
fn test_merge_change_fn() {
  let mut mutations: FastHashMap<u32, ValueChange<i32>> = FastHashMap::default();

  merge_change(&mut mutations, (1, ValueChange::Delta(10, Some(5))));
  assert_eq!(mutations.len(), 1);
  assert_eq!(mutations[&1], ValueChange::Delta(10, Some(5)));

  // merge second change to same key → keeps original previous, updates new
  merge_change(&mut mutations, (1, ValueChange::Delta(15, Some(10))));
  assert_eq!(mutations.len(), 1);
  assert_eq!(mutations[&1], ValueChange::Delta(15, Some(5)));
}

#[test]
fn test_merge_change_fn_cancel() {
  let mut mutations: FastHashMap<u32, ValueChange<i32>> = FastHashMap::default();

  merge_change(&mut mutations, (1, ValueChange::Delta(10, Some(5))));

  // second change reverses the first → cancels out entirely
  merge_change(&mut mutations, (1, ValueChange::Delta(5, Some(10))));
  assert!(mutations.is_empty());
}

#[test]
fn test_integrate_change_fn() {
  let mut state: FastHashMap<u32, i32> = FastHashMap::default();

  integrate_change(&mut state, (1, ValueChange::Delta(42, None)));
  assert_eq!(state[&1], 42);

  integrate_change(&mut state, (2, ValueChange::Delta(99, None)));
  assert_eq!(state[&2], 99);

  integrate_change(&mut state, (1, ValueChange::Remove(42)));
  assert!(!state.contains_key(&1));
  assert_eq!(state[&2], 99);
}

#[test]
fn test_validate_delta_fn() {
  let mut state: FastHashMap<u32, i32> = FastHashMap::default();
  state.insert(1, 10);
  state.insert(2, 20);

  let delta: FastHashMap<u32, ValueChange<i32>> = FastHashMap::from_iter([
    (1, ValueChange::Delta(15, Some(10))),
    (2, ValueChange::Remove(20)),
    (3, ValueChange::Delta(30, None)),
  ]);

  validate_delta(&mut state, false, "test", &delta);

  assert_eq!(state[&1], 15);
  assert!(!state.contains_key(&2));
  assert_eq!(state[&3], 30);
}
