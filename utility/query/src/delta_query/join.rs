use crate::*;

#[derive(Clone)]
pub struct CrossJoinValueChange<A, B, DA, DB> {
  pub a: DA,
  pub b: DB,
  pub a_current: A,
  pub b_current: B,
}

impl<A, B, DA, DB, K1, K2, V1, V2> Query for CrossJoinValueChange<A, B, DA, DB>
where
  K1: CKey,
  K2: CKey,
  V1: CValue,
  V2: CValue,
  DA: Query<Key = K1, Value = ValueChange<V1>>,
  DB: Query<Key = K2, Value = ValueChange<V2>>,
  A: Query<Key = K1, Value = V1>,
  B: Query<Key = K2, Value = V2>,
{
  type Key = (K1, K2);
  type Value = ValueChange<(V1, V2)>;

  fn iter_key_value(&self) -> impl Iterator<Item = ((K1, K2), ValueChange<(V1, V2)>)> + '_ {
    let cross_section = self.a.iter_key_value().flat_map(move |(k1, v1_change)| {
      self.b.iter_key_value().filter_map(move |(k2, v2_change)| {
        cross_join_change(
          &k1,
          &k2,
          Some(v1_change.clone()),
          Some(v2_change),
          &|k| self.a_current.access(k),
          &|k| self.b_current.access(k),
        )
        .and_then(exist_both)
        .map(|v| ((k1.clone(), k2), v))
      })
    });

    let a_iter = self.a.iter_key_value();
    let a_max = a_iter.size_hint().1;
    let a_side_change_with_b = a_iter.flat_map(move |(k1, v1_change)| {
      self
        .b_current
        .iter_key_value()
        .filter(move |(k2, _)| !self.b.contains(k2))
        .filter_map(move |(k2, _)| {
          cross_join_change(
            &k1,
            &k2,
            Some(v1_change.clone()),
            None,
            &|k| self.a_current.access(k),
            &|k| self.b_current.access(k),
          )
          .and_then(exist_both)
          .map(|v| ((k1.clone(), k2), v))
        })
    });

    let b_iter = self.b.iter_key_value();
    let b_max = b_iter.size_hint().1;
    let b_side_change_with_a = b_iter.flat_map(move |(k2, v2_change)| {
      self
        .a_current
        .iter_key_value()
        .filter(move |(k1, _)| !self.a.contains(k1))
        .filter_map(move |(k1, _)| {
          cross_join_change(
            &k1,
            &k2,
            None,
            Some(v2_change.clone()),
            &|k| self.a_current.access(k),
            &|k| self.b_current.access(k),
          )
          .and_then(exist_both)
          .map(|v| ((k1, k2.clone()), v))
        })
    });

    let iter = cross_section
      .chain(a_side_change_with_b)
      .chain(b_side_change_with_a);

    struct SizeHintOverride<T> {
      iter: T,
      max: Option<usize>,
    }

    impl<T: Iterator> Iterator for SizeHintOverride<T> {
      type Item = T::Item;

      fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
      }

      fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.max)
      }
    }

    let mut max = None;
    if let (Some(a_max), Some(b_max)) = (a_max, b_max) {
      let a_current_max = self.a_current.iter_key_value().size_hint().1;
      let b_current_max = self.b_current.iter_key_value().size_hint().1;
      if let (Some(a_current_max), Some(b_current_max)) = (a_current_max, b_current_max) {
        max = Some(a_max * b_current_max + b_max * a_current_max);
      }
    }

    SizeHintOverride { iter, max }
  }

  fn access(&self, (k1, k2): &(K1, K2)) -> Option<ValueChange<(V1, V2)>> {
    cross_join_change(
      &k1,
      &k2,
      self.a.access(k1),
      self.b.access(k2),
      &|k| self.a_current.access(k),
      &|k| self.b_current.access(k),
    )
    .and_then(exist_both)
  }

  fn has_item_hint(&self) -> bool {
    self.a.has_item_hint() || self.b.has_item_hint()
  }
}

fn cross_join_change<K1: Clone, K2: Clone, V1: Clone, V2: Clone>(
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
        let v1_current = v1_current(k1);
        ValueChange::Remove((v1_current, Some(p2)))
      }
    },
    (Some(change1), None) => match change1 {
      ValueChange::Delta(v1, p1) => {
        let v2_current = v2_current(k2);
        ValueChange::Delta((Some(v1), v2_current.clone()), Some((p1, v2_current)))
      }
      ValueChange::Remove(p1) => {
        let v2_current = v2_current(k2);
        ValueChange::Remove((Some(p1), v2_current))
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

fn exist_both<V1, V2>(
  change: ValueChange<(Option<V1>, Option<V2>)>,
) -> Option<ValueChange<(V1, V2)>> {
  match change {
    ValueChange::Delta(new, previous) => Some(ValueChange::Delta(
      new.0.zip(new.1)?,
      previous.and_then(|v| v.0.zip(v.1)),
    )),
    ValueChange::Remove((v1, v2)) => Some(ValueChange::Remove(v1.zip(v2)?)),
  }
}

// --- cross_join_change helper branch coverage ---

#[test]
fn test_cross_join_value_change_delta_delta() {
  let mut a_delta = FastHashMap::default();
  a_delta.insert(1u32, ValueChange::Delta(10i32, Some(5)));

  let mut b_delta = FastHashMap::default();
  b_delta.insert(100u32, ValueChange::Delta(20, Some(15)));

  let joined = CrossJoinValueChange {
    a: a_delta,
    b: b_delta,
    a_current: FastHashMap::from_iter([(1, 10)]),
    b_current: FastHashMap::from_iter([(100, 20)]),
  };

  validate_query_consistency(&joined);
  assert_eq!(
    joined.access(&(1, 100)).unwrap(),
    ValueChange::Delta((10, 20), Some((5, 15)))
  );
}

#[test]
fn test_cross_join_value_change_remove_remove() {
  let mut a_delta = FastHashMap::default();
  a_delta.insert(1u32, ValueChange::<i32>::Remove(5));

  let mut b_delta = FastHashMap::default();
  b_delta.insert(100u32, ValueChange::<i32>::Remove(15));

  let joined = CrossJoinValueChange {
    a: a_delta,
    b: b_delta,
    a_current: FastHashMap::default(),
    b_current: FastHashMap::default(),
  };

  validate_query_consistency(&joined);
  assert_eq!(
    joined.access(&(1, 100)).unwrap(),
    ValueChange::Remove((5, 15))
  );
}

#[test]
fn test_cross_join_value_change_delta_remove() {
  let mut a_delta = FastHashMap::default();
  a_delta.insert(1u32, ValueChange::Delta(10i32, Some(5)));

  let mut b_delta = FastHashMap::default();
  b_delta.insert(100u32, ValueChange::<i32>::Remove(15));

  // Delta + Remove: removed side has no current value, exist_both filters it out
  let joined = CrossJoinValueChange {
    a: a_delta,
    b: b_delta,
    a_current: FastHashMap::from_iter([(1, 10)]),
    b_current: FastHashMap::default(), // removed key should not be in current
  };

  validate_query_consistency(&joined);
  // Delta + Remove with None current → filtered out by exist_both
  assert_eq!(joined.access(&(1, 100)), None);
}

#[test]
fn test_cross_join_value_change_remove_delta() {
  let mut a_delta = FastHashMap::default();
  a_delta.insert(1u32, ValueChange::<i32>::Remove(5));

  let mut b_delta = FastHashMap::default();
  b_delta.insert(100u32, ValueChange::Delta(20, Some(15)));

  // Remove + Delta: removed side has no current value, exist_both filters it out
  let joined = CrossJoinValueChange {
    a: a_delta,
    b: b_delta,
    a_current: FastHashMap::default(), // removed key should not be in current
    b_current: FastHashMap::from_iter([(100, 20)]),
  };

  validate_query_consistency(&joined);
  // Remove + Delta with None current → filtered out by exist_both
  assert_eq!(joined.access(&(1, 100)), None);
}

// --- iter_key_value stream coverage ---

#[test]
fn test_cross_join_value_change_a_delta_b_static() {
  // a side has change, b side has NO change (a_side_change_with_b path)
  let mut a_delta = FastHashMap::default();
  a_delta.insert(1u32, ValueChange::Delta(10i32, Some(5)));

  // b has static entries NOT in delta
  let b_delta: FastHashMap<u32, ValueChange<i32>> = FastHashMap::default();
  let b_current = FastHashMap::from_iter([(100, 42), (200, 99)]);

  let joined = CrossJoinValueChange {
    a: a_delta,
    b: b_delta,
    a_current: FastHashMap::from_iter([(1, 10)]),
    b_current,
  };

  validate_query_consistency(&joined);
  // cross product: a change × b's static entries
  assert_eq!(
    joined.access(&(1, 100)).unwrap(),
    ValueChange::Delta((10, 42), Some((5, 42)))
  );
  assert_eq!(
    joined.access(&(1, 200)).unwrap(),
    ValueChange::Delta((10, 99), Some((5, 99)))
  );
}

#[test]
fn test_cross_join_value_change_b_delta_a_static() {
  // b side has change, a side has NO change (b_side_change_with_a path)
  let a_delta: FastHashMap<u32, ValueChange<i32>> = FastHashMap::default();

  let mut b_delta = FastHashMap::default();
  b_delta.insert(100u32, ValueChange::Delta(20, Some(15)));

  let a_current = FastHashMap::from_iter([(1, 10), (2, 99)]);

  let joined = CrossJoinValueChange {
    a: a_delta,
    b: b_delta,
    a_current,
    b_current: FastHashMap::from_iter([(100, 20)]),
  };

  validate_query_consistency(&joined);
  assert_eq!(
    joined.access(&(1, 100)).unwrap(),
    ValueChange::Delta((10, 20), Some((10, 15)))
  );
  assert_eq!(
    joined.access(&(2, 100)).unwrap(),
    ValueChange::Delta((99, 20), Some((99, 15)))
  );
}

#[test]
fn test_cross_join_value_change_mixed() {
  // Delta + Delta, Delta + Remove, static entries: all three streams exercised
  let mut a_delta = FastHashMap::default();
  a_delta.insert(1u32, ValueChange::Delta(10i32, Some(5))); // Delta
  a_delta.insert(2, ValueChange::Remove(7)); // Remove (a_current has no key 2)

  let mut b_delta = FastHashMap::default();
  b_delta.insert(100u32, ValueChange::Delta(20, Some(15))); // Delta

  // key 2 was removed, so a_current doesn't have it; key 3 is static
  let a_current = FastHashMap::from_iter([(1, 10), (3, 99)]);
  let b_current = FastHashMap::from_iter([(100, 20), (200, 42)]); // key 200 is static

  let joined = CrossJoinValueChange {
    a: a_delta,
    b: b_delta,
    a_current,
    b_current,
  };

  validate_query_consistency(&joined);

  // cross_section: a_delta × b_delta
  // (1 Delta, 100 Delta) → Delta
  assert_eq!(
    joined.access(&(1, 100)).unwrap(),
    ValueChange::Delta((10, 20), Some((5, 15)))
  );
  // (2 Remove, 100 Delta) → v1_current(2)=None → exist_both returns None → filtered out

  // a_side_change_with_b: a_delta × b_static (200 not in b_delta)
  assert_eq!(
    joined.access(&(1, 200)).unwrap(),
    ValueChange::Delta((10, 42), Some((5, 42)))
  );
  // key 2 (Remove) × b_static 200 → Remove
  assert_eq!(
    joined.access(&(2, 200)).unwrap(),
    ValueChange::Remove((7, 42))
  );

  // b_side_change_with_a: b_delta × a_static (3 not in a_delta)
  assert_eq!(
    joined.access(&(3, 100)).unwrap(),
    ValueChange::Delta((99, 20), Some((99, 15)))
  );
}
