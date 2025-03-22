use crate::*;

pub trait ReactiveValueRefQuery:
  ReactiveQuery<Compute: QueryCompute<View: DynValueRefQuery<Key = Self::Key, Value = Self::Value>>>
{
  fn into_reactive_state_self_contained(
    self,
  ) -> impl ReactiveGeneralQuery<Output = Box<dyn std::any::Any>>
  where
    Self: Sized + 'static,
  {
    ReactiveValueRefQueryAsReactiveGeneralQuery { inner: self }
  }

  fn into_boxed_self_contain(self) -> BoxedDynReactiveValueRefQuery<Self::Key, Self::Value>
  where
    Self: Sized + 'static,
  {
    Box::new(self)
  }
}
impl<T> ReactiveValueRefQuery for T
where
  T: ReactiveQuery,
  <T::Compute as QueryCompute>::View: DynValueRefQuery<Key = T::Key, Value = T::Value>,
{
}

pub type BoxedDynReactiveValueRefQuery<K, V> =
  Box<dyn DynReactiveValueRefQuery<Key = K, Value = V>>;
pub type DynReactiveValueRefQueryPoll<K, V> = (
  BoxedDynQuery<K, ValueChange<V>>,
  Box<dyn DynValueRefQuery<Key = K, Value = V>>,
);

pub trait DynReactiveValueRefQuery {
  type Key: CKey;
  type Value: CValue;
  fn poll_changes_self_contained_dyn(
    &self,
    cx: &mut Context,
  ) -> DynReactiveValueRefQueryPoll<Self::Key, Self::Value>;

  fn extra_request_dyn(&mut self, request: &mut ReactiveQueryRequest);
}

impl<T> DynReactiveValueRefQuery for T
where
  T: ReactiveValueRefQuery,
{
  type Key = T::Key;
  type Value = T::Value;
  fn poll_changes_self_contained_dyn(
    &self,
    cx: &mut Context,
  ) -> DynReactiveValueRefQueryPoll<Self::Key, Self::Value> {
    let (d, v) = self.describe(cx).resolve();
    (Box::new(d), Box::new(v))
  }

  fn extra_request_dyn(&mut self, request: &mut ReactiveQueryRequest) {
    self.request(request)
  }
}
