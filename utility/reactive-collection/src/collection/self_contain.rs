use crate::*;

pub trait ReactiveCollectionSelfContained:
  ReactiveCollection<View: DynVirtualCollectionSelfContained<Key = Self::Key, Value = Self::Value>>
{
  fn into_reactive_state_self_contained(self) -> impl ReactiveQuery<Output = Box<dyn std::any::Any>>
  where
    Self: Sized + 'static,
  {
    ReactiveCollectionSelfContainedAsReactiveQuery { inner: self }
  }

  fn into_boxed_self_contain(
    self,
  ) -> BoxedDynReactiveCollectionSelfContained<Self::Key, Self::Value>
  where
    Self: Sized + 'static,
  {
    Box::new(self)
  }
}
impl<T> ReactiveCollectionSelfContained for T
where
  T: ReactiveCollection,
  T::View: DynVirtualCollectionSelfContained<Key = T::Key, Value = T::Value>,
{
}

pub type BoxedDynReactiveCollectionSelfContained<K, V> =
  Box<dyn DynReactiveCollectionSelfContained<Key = K, Value = V>>;
pub type DynReactiveCollectionSelfContainedPoll<K, V> = (
  BoxedDynVirtualCollection<K, ValueChange<V>>,
  Box<dyn DynVirtualCollectionSelfContained<Key = K, Value = V>>,
);

pub trait DynReactiveCollectionSelfContained {
  type Key: CKey;
  type Value: CValue;
  fn poll_changes_self_contained_dyn(
    &self,
    cx: &mut Context,
  ) -> DynReactiveCollectionSelfContainedPoll<Self::Key, Self::Value>;

  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation);
}

impl<T> DynReactiveCollectionSelfContained for T
where
  T: ReactiveCollectionSelfContained,
{
  type Key = T::Key;
  type Value = T::Value;
  fn poll_changes_self_contained_dyn(
    &self,
    cx: &mut Context,
  ) -> DynReactiveCollectionSelfContainedPoll<Self::Key, Self::Value> {
    let (d, v) = self.poll_changes(cx);
    (Box::new(d), Box::new(v))
  }

  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation) {
    self.extra_request(request)
  }
}
