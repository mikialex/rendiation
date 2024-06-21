use crate::*;

pub trait ReactiveCollectionSelfContained<K: CKey, V: CValue>: ReactiveCollection<K, V> {
  fn access_ref_collection(&self) -> Box<dyn VirtualCollectionSelfContained<K, V>>;

  fn into_reactive_state_self_contained(self) -> impl ReactiveQuery<Output = Box<dyn std::any::Any>>
  where
    Self: Sized + 'static,
  {
    ReactiveCollectionSelfContainedAsReactiveQuery {
      inner: self,
      phantom: PhantomData,
    }
  }

  fn into_boxed_self_contain(self) -> Box<dyn ReactiveCollectionSelfContained<K, V>>
  where
    Self: Sized + 'static,
  {
    Box::new(self)
  }
}

impl<K: CKey, V: CValue> ReactiveCollectionSelfContained<K, V> for () {
  fn access_ref_collection(&self) -> Box<dyn VirtualCollectionSelfContained<K, V>> {
    Box::new(())
  }
}
