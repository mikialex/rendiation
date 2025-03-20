use crate::*;

#[derive(Clone)]
pub struct UnionQuery<A, B, F> {
  pub a: A,
  pub b: B,
  pub f: F,
}

impl<A, B, F, O> Query for UnionQuery<A, B, F>
where
  A: Query,
  B: Query<Key = A::Key>,
  F: Fn((Option<A::Value>, Option<B::Value>)) -> Option<O> + Send + Sync + Clone + 'static,

  O: CValue,
{
  type Key = A::Key;
  type Value = O;
  fn iter_key_value(&self) -> impl Iterator<Item = (A::Key, O)> + '_ {
    let a_side = self
      .a
      .iter_key_value()
      .filter_map(|(k, v1)| (self.f)((Some(v1), self.b.access(&k))).map(|v| (k, v)));

    let b_side = self
      .b
      .iter_key_value()
      .filter(|(k, _)| self.a.access(k).is_none()) // remove the a_side part
      .filter_map(|(k, v2)| (self.f)((self.a.access(&k), Some(v2))).map(|v| (k, v)));

    avoid_huge_debug_symbols_by_boxing_iter(a_side.chain(b_side))
  }

  fn access(&self, key: &A::Key) -> Option<O> {
    (self.f)((self.a.access(key), self.b.access(key)))
  }
}
