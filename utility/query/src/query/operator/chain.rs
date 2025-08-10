use crate::*;

#[derive(Clone)]
pub struct ChainQuery<R, U> {
  pub first: R,
  pub next: U,
}

impl<U, R> Query for ChainQuery<R, U>
where
  U: Query,
  R: Query<Value = U::Key>,
{
  type Key = R::Key;
  type Value = U::Value;
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    // this is pretty costly
    self
      .first
      .iter_key_value()
      .filter_map(|(k, _v)| self.access(&k).map(|v| (k, v)))
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    let o = self.first.access(key)?;
    self.next.access(&o)
  }
}
