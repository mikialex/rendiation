use futures::FutureExt;

use crate::*;

pub trait ReactiveCollectionSelfContained<K: CKey, V: CValue>:
  ReactiveCollection<K, V, View: VirtualCollectionSelfContained<K, V>>
{
  fn into_reactive_state_self_contained(self) -> impl ReactiveQuery<Output = Box<dyn std::any::Any>>
  where
    Self: Sized + 'static,
  {
    ReactiveCollectionSelfContainedAsReactiveQuery {
      inner: self,
      phantom: PhantomData,
    }
  }

  fn into_boxed_self_contain(self) -> Box<dyn DynReactiveCollectionSelfContained<K, V>>
  where
    Self: Sized + 'static,
  {
    Box::new(self)
  }
}
impl<K, V, T> ReactiveCollectionSelfContained<K, V> for T
where
  K: CKey,
  V: CValue,
  T: ReactiveCollection<K, V>,
  T::View: VirtualCollectionSelfContained<K, V>,
{
}

type DynReactiveCollectionSelfContainedPollResult<K, V> = Pin<
  Box<
    dyn Future<
      Output = (
        Box<dyn DynVirtualCollection<K, ValueChange<V>>>,
        Box<dyn VirtualCollectionSelfContained<K, V>>,
      ),
    >,
  >,
>;

pub trait DynReactiveCollectionSelfContained<K: CKey, V: CValue> {
  fn poll_changes_self_contained_dyn(
    &self,
    cx: &mut Context,
  ) -> DynReactiveCollectionSelfContainedPollResult<K, V>;

  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation);
}

impl<K, V, T> DynReactiveCollectionSelfContained<K, V> for T
where
  K: CKey,
  V: CValue,
  T: ReactiveCollectionSelfContained<K, V>,
{
  fn poll_changes_self_contained_dyn(
    &self,
    cx: &mut Context,
  ) -> DynReactiveCollectionSelfContainedPollResult<K, V> {
    let task = self.poll_changes(cx);
    task
      .map(|(d, v)| {
        (
          Box::new(d) as Box<dyn DynVirtualCollection<K, ValueChange<V>>>,
          Box::new(v) as Box<dyn VirtualCollectionSelfContained<K, V>>,
        )
      })
      .boxed()
  }

  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation) {
    self.extra_request(request)
  }
}
