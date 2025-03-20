use crate::*;

#[derive(Clone)]
pub struct CrossJoinQuery<A, B> {
  pub a: A,
  pub b: B,
}

impl<A, B> Query for CrossJoinQuery<A, B>
where
  A: Query,
  B: Query,
{
  type Key = (A::Key, B::Key);
  type Value = (A::Value, B::Value);
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self.a.iter_key_value().flat_map(move |(k1, v1)| {
      self
        .b
        .iter_key_value()
        .map(move |(k2, v2)| ((k1.clone(), k2), (v1.clone(), v2)))
    })
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    self.a.access(&key.0).zip(self.b.access(&key.1))
  }
}
