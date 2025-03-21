use crate::*;

impl<A, B> ReactiveQuery for CrossJoinQuery<A, B>
where
  A: ReactiveQuery,
  B: ReactiveQuery,
{
  type Key = (A::Key, B::Key);
  type Value = (A::Value, B::Value);
  type Changes = impl Query<Key = Self::Key, Value = ValueChange<Self::Value>>;
  type View = impl Query<Key = Self::Key, Value = Self::Value>;
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (t1, a_access) = self.a.poll_changes(cx);
    let (t2, b_access) = self.b.poll_changes(cx);

    let a_access = a_access;
    let b_access = b_access;

    let d = CrossJoinValueChange {
      a: t1,
      b: t2,
      a_current: a_access.clone(),
      b_current: b_access.clone(),
    };

    let v = CrossJoinQuery {
      a: a_access,
      b: b_access,
    };

    (d, v)
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.a.request(request);
    self.b.request(request);
  }
}

#[derive(Clone)]
struct CrossJoinValueChange<A, B, DA, DB> {
  a: DA,
  b: DB,
  a_current: A,
  b_current: B,
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
      self.b.iter_key_value().map(move |(k2, v2_change)| {
        join_change(
          &k1,
          &k2,
          Some(v1_change.clone()),
          Some(v2_change),
          &|k| self.a_current.access(k),
          &|k| self.b_current.access(k),
        )
        .map(|v| ((k1.clone(), k2), exist_both(v).unwrap()))
        .unwrap()
      })
    });

    let a_side_change_with_b = self.a.iter_key_value().flat_map(move |(k1, v1_change)| {
      self
        .b_current
        .iter_key_value()
        .filter(move |(k2, _)| !self.b.contains(k2))
        .map(move |(k2, _)| {
          join_change(
            &k1,
            &k2,
            Some(v1_change.clone()),
            None,
            &|k| self.a_current.access(k),
            &|k| self.b_current.access(k),
          )
          .map(|v| ((k1.clone(), k2), exist_both(v).unwrap()))
          .unwrap()
        })
    });

    let b_side_change_with_a = self.b.iter_key_value().flat_map(move |(k2, v2_change)| {
      self
        .a_current
        .iter_key_value()
        .filter(move |(k1, _)| !self.a.contains(k1))
        .map(move |(k1, _)| {
          join_change(
            &k1,
            &k2,
            None,
            Some(v2_change.clone()),
            &|k| self.a_current.access(k),
            &|k| self.b_current.access(k),
          )
          .map(|v| ((k1, k2.clone()), exist_both(v).unwrap()))
          .unwrap()
        })
    });

    cross_section
      .chain(a_side_change_with_b)
      .chain(b_side_change_with_a)
  }

  fn access(&self, (k1, k2): &(K1, K2)) -> Option<ValueChange<(V1, V2)>> {
    join_change(
      &k1,
      &k2,
      self.a.access(k1),
      self.b.access(k2),
      &|k| self.a_current.access(k),
      &|k| self.b_current.access(k),
    )
    .and_then(exist_both)
  }
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

pub fn join_change<K1: Clone, K2: Clone, V1: Clone, V2: Clone>(
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
        ValueChange::Delta((Some(v1), v2_current(k2)), Some((p1, Some(p2))))
      }
      (ValueChange::Remove(p1), ValueChange::Delta(v2, p2)) => {
        ValueChange::Delta((v1_current(k1), Some(v2)), Some((Some(p1), p2)))
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
