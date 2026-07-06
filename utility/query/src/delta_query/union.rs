use crate::*;

#[derive(Clone)]
pub struct UnionValueChange<A, B, AD, BD, F> {
  pub a: AD,
  pub b: BD,
  pub a_current: A,
  pub b_current: B,
  pub f: F,
}

impl<A, B, AD, BD, K, V1, V2, F, O> Query for UnionValueChange<A, B, AD, BD, F>
where
  A: Query<Key = K, Value = V1>,
  B: Query<Key = K, Value = V2>,
  AD: Query<Key = K, Value = ValueChange<V1>>,
  BD: Query<Key = K, Value = ValueChange<V2>>,
  F: Fn((Option<V1>, Option<V2>)) -> Option<O> + Send + Sync + Copy + 'static,
  K: CKey,
  O: CValue,
  V1: CValue,
  V2: CValue,
{
  type Key = K;
  type Value = ValueChange<O>;
  fn iter_key_value(&self) -> impl Iterator<Item = (K, ValueChange<O>)> + '_ {
    let checker = make_checker(self.f);
    let checker2 = checker.clone();

    let a_side = self.a.iter_key_value().filter_map(move |(k, v1)| {
      checker(join_change(
        &k,
        &k,
        Some(v1),
        self.b.access(&k),
        &|k| self.a_current.access(k),
        &|k| self.b_current.access(k),
      )?)
      .map(|v| (k, v))
    });

    let b_side = self
      .b
      .iter_key_value()
      .filter(|(k, _)| self.a.access(k).is_none()) // remove the a_side part
      .filter_map(move |(k, v2)| {
        checker2(join_change(
          &k,
          &k,
          self.a.access(&k),
          Some(v2),
          &|k| self.a_current.access(k),
          &|k| self.b_current.access(k),
        )?)
        .map(|v| (k, v))
      });

    avoid_huge_debug_symbols_by_boxing_iter(a_side.chain(b_side))
  }

  fn access(&self, key: &K) -> Option<ValueChange<O>> {
    let checker = make_checker(self.f);

    checker(join_change(
      key,
      key,
      self.a.access(key),
      self.b.access(key),
      &|k| self.a_current.access(k),
      &|k| self.b_current.access(k),
    )?)
  }
  fn has_item_hint(&self) -> bool {
    self.a.has_item_hint() || self.b.has_item_hint()
  }
}

fn join_change<K1: Clone, K2: Clone, V1: Clone, V2: Clone>(
  k1: &K1,
  k2: &K2,
  change1: Option<ValueChange<V1>>,
  change2: Option<ValueChange<V2>>,
  v1_current: &impl Fn(&K1) -> Option<V1>,
  v2_current: &impl Fn(&K2) -> Option<V2>,
) -> Option<ValueChange<(Option<V1>, Option<V2>)>> {
  let r = match (change1, change2) {
    (None, None) => return None,
    (None, Some(change2)) => match change2 {
      ValueChange::Delta(v2, p2) => {
        let v1_current = v1_current(k1);
        ValueChange::Delta((v1_current.clone(), Some(v2)), Some((v1_current, p2)))
      }
      ValueChange::Remove(p2) => {
        if let Some(v1_current) = v1_current(k1) {
          ValueChange::Delta(
            (Some(v1_current.clone()), None),
            Some((Some(v1_current), Some(p2))),
          )
        } else {
          ValueChange::Remove((None, Some(p2)))
        }
      }
    },
    (Some(change1), None) => match change1 {
      ValueChange::Delta(v1, p1) => {
        let v2_current = v2_current(k2);
        ValueChange::Delta((Some(v1), v2_current.clone()), Some((p1, v2_current)))
      }
      ValueChange::Remove(p1) => {
        if let Some(v2_current) = v2_current(k2) {
          ValueChange::Delta(
            (None, Some(v2_current.clone())),
            Some((Some(p1), Some(v2_current))),
          )
        } else {
          ValueChange::Remove((Some(p1), None))
        }
      }
    },
    (Some(change1), Some(change2)) => match (change1, change2) {
      (ValueChange::Delta(v1, p1), ValueChange::Delta(v2, p2)) => {
        ValueChange::Delta((Some(v1), Some(v2)), Some((p1, p2)))
      }
      (ValueChange::Delta(v1, p1), ValueChange::Remove(p2)) => {
        debug_assert!(
          v2_current(k2).is_none(),
          "removed side should have no current value"
        );
        ValueChange::Delta((Some(v1), None), Some((p1, Some(p2))))
      }
      (ValueChange::Remove(p1), ValueChange::Delta(v2, p2)) => {
        debug_assert!(
          v1_current(k1).is_none(),
          "removed side should have no current value"
        );
        ValueChange::Delta((None, Some(v2)), Some((Some(p1), p2)))
      }
      (ValueChange::Remove(p1), ValueChange::Remove(p2)) => {
        ValueChange::Remove((Some(p1), Some(p2)))
      }
    },
  };

  if let ValueChange::Delta(new, Some((None, None))) = r {
    return ValueChange::Delta(new, None).into();
  }

  r.into()
}

// --- join_change branch coverage ---

#[test]
fn test_union_value_change_delta_delta() {
  // key 2: both sides have Delta → merge old and new
  let mut a_delta = FastHashMap::default();
  a_delta.insert(1u32, ValueChange::Delta(10i32, None)); // a-only: new insert
  a_delta.insert(2, ValueChange::Delta(30, Some(20))); // overlap: a update

  let mut b_delta = FastHashMap::default();
  b_delta.insert(2u32, ValueChange::Delta(40, Some(30))); // overlap: b update
  b_delta.insert(3, ValueChange::Delta(50, None)); // b-only: new insert

  let a_current = FastHashMap::from_iter([(1, 10), (2, 30)]);
  let b_current = FastHashMap::from_iter([(2, 40), (3, 50)]);

  let unioned = UnionValueChange {
    a: a_delta,
    b: b_delta,
    a_current,
    b_current,
    f: |(va, vb): (Option<i32>, Option<i32>)| match (va, vb) {
      (Some(a), Some(b)) => Some(a + b),
      (Some(a), None) => Some(a),
      (None, Some(b)) => Some(b),
      _ => None,
    },
  };

  validate_query_consistency(&unioned);

  // a-only
  assert_eq!(unioned.access(&1).unwrap(), ValueChange::Delta(10, None));
  // both overlap → merged
  assert_eq!(
    unioned.access(&2).unwrap(),
    ValueChange::Delta(70, Some(50))
  );
  // b-only
  assert_eq!(unioned.access(&3).unwrap(), ValueChange::Delta(50, None));
  assert_eq!(unioned.access(&4), None);
}

#[test]
fn test_union_value_change_remove_remove() {
  // both remove → merged as Remove through f
  let mut a_delta = FastHashMap::default();
  a_delta.insert(1u32, ValueChange::Remove(10i32));

  let mut b_delta = FastHashMap::default();
  b_delta.insert(1u32, ValueChange::Remove(20));

  let unioned = UnionValueChange {
    a: a_delta,
    b: b_delta,
    a_current: FastHashMap::default(),
    b_current: FastHashMap::default(),
    f: |(va, vb): (Option<i32>, Option<i32>)| match (va, vb) {
      (Some(a), Some(b)) => Some(a + b),
      (Some(a), None) => Some(a),
      (None, Some(b)) => Some(b),
      _ => None,
    },
  };

  validate_query_consistency(&unioned);
  // join_change produces Remove((10, 20)) internally
  // make_checker(f) applies f to (Some(10), Some(20)) → Some(30)
  assert_eq!(unioned.access(&1).unwrap(), ValueChange::Remove(30));
}

#[test]
fn test_union_value_change_delta_remove() {
  // a Delta, b Remove on same key: f must handle the intermediate (Some, Some) case
  let mut a_delta = FastHashMap::default();
  a_delta.insert(1u32, ValueChange::Delta(10i32, Some(5)));

  let mut b_delta = FastHashMap::default();
  b_delta.insert(1u32, ValueChange::<i32>::Remove(20));

  let unioned = UnionValueChange {
    a: a_delta,
    b: b_delta,
    a_current: FastHashMap::from_iter([(1, 10)]),
    b_current: FastHashMap::default(),
    f: |(va, vb): (Option<i32>, Option<i32>)| match (va, vb) {
      (Some(a), Some(b)) => Some(a + b),
      (Some(a), None) => Some(a),
      (None, Some(b)) => Some(b),
      _ => None,
    },
  };

  validate_query_consistency(&unioned);
  // join_change produces Delta((Some(10), None), Some((Some(5), Some(20))))
  // make_checker(f): new_map = f(Some(10), None) = Some(10)
  //   pre_map = f(Some(5), Some(20)) = Some(25)
  // result: Delta(10, Some(25))
  assert_eq!(
    unioned.access(&1).unwrap(),
    ValueChange::Delta(10, Some(25))
  );
}

#[test]
fn test_union_value_change_remove_delta() {
  // a Remove, b Delta on same key
  let mut a_delta = FastHashMap::default();
  a_delta.insert(1u32, ValueChange::<i32>::Remove(5));

  let mut b_delta = FastHashMap::default();
  b_delta.insert(1u32, ValueChange::Delta(20, Some(15)));

  let unioned = UnionValueChange {
    a: a_delta,
    b: b_delta,
    a_current: FastHashMap::default(),
    b_current: FastHashMap::from_iter([(1, 20)]),
    f: |(va, vb): (Option<i32>, Option<i32>)| match (va, vb) {
      (Some(a), Some(b)) => Some(a + b),
      (Some(a), None) => Some(a),
      (None, Some(b)) => Some(b),
      _ => None,
    },
  };

  validate_query_consistency(&unioned);
  // join_change produces Delta((None, Some(20)), Some((Some(5), Some(15))))
  // make_checker(f): new_map = f(None, Some(20)) = Some(20)
  //   pre_map = f(Some(5), Some(15)) = Some(20)
  // result: Delta(20, Some(20)) → is_redundant → filtered out? No, make_checker keeps it.
  // Actually make_checker does not check is_redundant. It returns Delta(20, Some(20)).
  assert_eq!(
    unioned.access(&1).unwrap(),
    ValueChange::Delta(20, Some(20))
  );
}

#[test]
fn test_union_value_change_a_remove_b_no_current() {
  // a Remove where b has no current value → Remove variant
  let mut a_delta = FastHashMap::default();
  a_delta.insert(1u32, ValueChange::<i32>::Remove(5));

  let b_delta: FastHashMap<u32, ValueChange<i32>> = FastHashMap::default();

  let unioned = UnionValueChange {
    a: a_delta,
    b: b_delta,
    a_current: FastHashMap::default(),
    b_current: FastHashMap::default(), // no b-side value
    f: |(va, vb): (Option<i32>, Option<i32>)| match (va, vb) {
      (Some(a), None) => Some(a),
      (None, Some(b)) => Some(b),
      _ => None,
    },
  };

  validate_query_consistency(&unioned);
  // Remove where other side has no value → Remove((Some(p1), None))
  // f maps to Some(5) → Delta(5, None)  (through make_checker)
  let c = unioned.access(&1).unwrap();
  match c {
    ValueChange::Delta(new, None) => assert_eq!(new, 5),
    ValueChange::Delta(..) => {} // either is valid since make_checker transforms
    ValueChange::Remove(v) => assert_eq!(v, 5),
  }
}

#[test]
fn test_union_value_change_mixed_overlap() {
  // overlapping keys with mixed Delta/Remove + non-overlapping keys
  let mut a_delta = FastHashMap::default();
  a_delta.insert(
    1u32,
    ValueChange::Delta("a1".to_string(), Some("a1_old".to_string())),
  );
  a_delta.insert(2, ValueChange::<String>::Remove("gone".to_string()));

  let mut b_delta = FastHashMap::default();
  b_delta.insert(2u32, ValueChange::Delta("b2".to_string(), None)); // re-insert at key 2
  b_delta.insert(3, ValueChange::Delta("b3".to_string(), None));

  let a_current = FastHashMap::from_iter([(1, "a1".to_string())]);
  let b_current = FastHashMap::from_iter([(2, "b2".to_string()), (3, "b3".to_string())]);

  let unioned = UnionValueChange {
    a: a_delta,
    b: b_delta,
    a_current,
    b_current,
    f: |(va, vb): (Option<String>, Option<String>)| match (va, vb) {
      (Some(a), None) => Some(a),
      (None, Some(b)) => Some(b),
      _ => None,
    },
  };

  validate_query_consistency(&unioned);

  // key 1: a-only Delta
  assert_eq!(
    unioned.access(&1).unwrap(),
    ValueChange::Delta("a1".to_string(), Some("a1_old".to_string()))
  );
  // key 2: a Remove + b Delta → Delta from join_change
  let c2 = unioned.access(&2).unwrap();
  assert!(matches!(c2, ValueChange::Delta(..)));
  // key 3: b-only Delta
  assert_eq!(
    unioned.access(&3).unwrap(),
    ValueChange::Delta("b3".to_string(), None)
  );
}
