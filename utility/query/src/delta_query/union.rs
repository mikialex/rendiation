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
