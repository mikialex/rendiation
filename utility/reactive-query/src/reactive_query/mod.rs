use crate::*;

mod self_contain;
pub use self_contain::*;

mod dyn_impl;
pub use dyn_impl::*;

mod operator;
pub use operator::*;

pub enum ReactiveQueryRequest {
  MemoryShrinkToFit,
}

pub trait ReactiveQuery: Sync + Send + 'static {
  type Key: CKey;
  type Value: CValue;
  type Compute: ReactiveQueryCompute<Key = Self::Key, Value = Self::Value>;

  fn poll_changes(&self, cx: &mut Context) -> Self::Compute;

  fn request(&mut self, request: &mut ReactiveQueryRequest);
}

pub trait ReactiveQueryCompute: Sync + Send + 'static {
  type Key: CKey;
  type Value: CValue;
  type Changes: Query<Key = Self::Key, Value = ValueChange<Self::Value>> + 'static;
  type View: Query<Key = Self::Key, Value = Self::Value> + 'static;

  fn resolve(&mut self) -> (Self::Changes, Self::View);
}

impl<K, V, Change, View> ReactiveQueryCompute for (Change, View)
where
  K: CKey,
  V: CValue,
  Change: Query<Key = K, Value = ValueChange<V>> + 'static,
  View: Query<Key = K, Value = V> + 'static,
{
  type Key = K;
  type Value = V;
  type Changes = Change;
  type View = View;
  fn resolve(&mut self) -> (Self::Changes, Self::View) {
    (self.0.clone(), self.1.clone())
  }
}

impl<K: CKey, V: CValue> ReactiveQuery for EmptyQuery<K, V> {
  type Key = K;
  type Value = V;
  type Compute = (EmptyQuery<K, ValueChange<V>>, EmptyQuery<K, V>);
  fn poll_changes(&self, _: &mut Context) -> Self::Compute {
    (Default::default(), Default::default())
  }
  fn request(&mut self, _: &mut ReactiveQueryRequest) {}
}
