use std::any::Any;

use crate::*;

mod map;
pub use map::*;
mod filter;
pub use filter::*;
mod join;
pub use join::*;
mod union;
pub use union::*;
mod chain;
pub use chain::*;

pub trait QueryExt: Query + Sized {
  fn into_boxed(self) -> BoxedDynQuery<Self::Key, Self::Value>
  where
    Self: 'static,
  {
    Arc::new(self)
  }

  fn keep_sth<X: Any + Send + Sync>(self, sth: X) -> KeptQuery<Self> {
    KeptQuery {
      query: self,
      holder: Arc::new(sth),
    }
  }

  fn map_value<V2, F: Fn(Self::Value) -> V2>(self, f: F) -> MappedValueQuery<Self, F> {
    MappedValueQuery {
      base: self,
      mapper: f,
    }
  }

  fn map<V2, F>(self, mapper: F) -> MappedQuery<Self, F>
  where
    F: Fn(&Self::Key, Self::Value) -> V2,
  {
    MappedQuery { base: self, mapper }
  }

  fn key_as_value(self) -> impl Query<Key = Self::Key, Value = Self::Key> {
    self.map(|k, _| k.clone())
  }

  fn filter_map<V2, F>(self, mapper: F) -> FilterMapQuery<Self, F>
  where
    F: Fn(Self::Value) -> Option<V2>,
  {
    FilterMapQuery { base: self, mapper }
  }

  fn key_dual_map_partial<K2, F1, F2>(self, f1: F1, f2: F2) -> KeyDualMappedQuery<Self, F1, F2>
  where
    F1: Fn(Self::Key) -> K2,
    F2: Fn(K2) -> Option<Self::Key>,
  {
    KeyDualMappedQuery { base: self, f1, f2 }
  }

  fn key_dual_map<K2, F1, F2>(
    self,
    f1: F1,
    f2: F2,
  ) -> KeyDualMappedQuery<Self, F1, AutoSomeFnResult<F2>>
  where
    K2: CKey,
    F1: Fn(Self::Key) -> K2,
    F2: Fn(K2) -> Self::Key,
  {
    self.key_dual_map_partial(f1, AutoSomeFnResult(f2))
  }

  fn chain<Q>(self, next: Q) -> ChainQuery<Self, Q> {
    ChainQuery { first: self, next }
  }
}
impl<T: ?Sized> QueryExt for T where Self: Query + Sized {}

#[derive(Clone, Copy)]
pub struct AutoSomeFnResult<F>(pub F);
impl<K, K2, F: FnOnce(K) -> K2> FnOnce<(K,)> for AutoSomeFnResult<F> {
  type Output = Option<K2>;

  extern "rust-call" fn call_once(self, args: (K,)) -> Self::Output {
    Some(self.0(args.0))
  }
}
impl<K, K2, F: FnMut(K) -> K2> FnMut<(K,)> for AutoSomeFnResult<F> {
  extern "rust-call" fn call_mut(&mut self, args: (K,)) -> Self::Output {
    Some(self.0(args.0))
  }
}
impl<K, K2, F: Fn(K) -> K2> Fn<(K,)> for AutoSomeFnResult<F> {
  extern "rust-call" fn call(&self, args: (K,)) -> Self::Output {
    Some(self.0(args.0))
  }
}

pub fn validate_query_consistency<Q: Query>(query: &Q) {
  let pairs: Vec<(Q::Key, Q::Value)> = query.iter_key_value().collect();

  // verify iter_key_value and access consistency: each key's access result must match the iteration
  for (key, expected_value) in &pairs {
    let accessed = query.access(key);
    assert_eq!(
      accessed.as_ref(),
      Some(expected_value),
      "access returned different value than iter_key_value for key {:?}",
      key
    );
    assert!(
      query.contains(key),
      "contains should return true for key {:?} that exists in iter_key_value",
      key
    );
  }

  // verify has_item_hint correctness: when it returns false, the query must be truly empty
  if !query.has_item_hint() {
    assert!(
      pairs.is_empty(),
      "has_item_hint returned false but iter_key_value returned items"
    );
  }
}
