use crate::*;

#[derive(Clone)]
pub struct Select<T>(pub T);

impl<T> Query for Select<T>
where
  T: IteratorProvider + Clone + Send + Sync,
  T::Item: Query,
{
  type Key = <T::Item as Query>::Key;

  type Value = <T::Item as Query>::Value;

  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self.0.create_iter().flat_map(|q| q.iter_key_value())
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    for q in self.0.create_iter() {
      if let Some(v) = q.access(key) {
        return Some(v);
      }
    }
    None
  }
}

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
