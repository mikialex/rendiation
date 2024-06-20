use crate::*;

pub trait ReactiveState {
  type State;
  fn poll_current(&mut self, cx: &mut Context) -> Self::State;
}

// another design direction:
//
// pub trait ReactiveAsyncState: ReactiveState<State: Future<Output = Self::AsyncState>> {
//   type AsyncState;
// }

// pub trait ReactiveCollectiveUpdate<K: CKey, V: CValue> {
//   type Current: VirtualCollection<K, V>;
//   type Delta: VirtualCollection<K, ValueChange<V>>;
// }

// pub trait ReactiveCollectionImproved<K: CKey, V: CValue>:
//   ReactiveAsyncState<AsyncState: ReactiveCollectiveUpdate<K, V>>
// {
// }

pub struct ReactiveCollectionAsReactiveState<K, V, T> {
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, V, T> ReactiveState for ReactiveCollectionAsReactiveState<K, V, T>
where
  K: CKey,
  V: CValue,
  T: ReactiveCollection<K, V>,
{
  type State = Box<dyn std::any::Any>;

  fn poll_current(&mut self, cx: &mut Context) -> Self::State {
    let _ = self.inner.poll_changes(cx);
    Box::new(self.inner.access())
  }
}

pub struct ReactiveCollectionSelfContainedAsReactiveState<K, V, T> {
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, V, T> ReactiveState for ReactiveCollectionSelfContainedAsReactiveState<K, V, T>
where
  K: CKey,
  V: CValue,
  T: ReactiveCollectionSelfContained<K, V>,
{
  type State = Box<dyn std::any::Any>;

  fn poll_current(&mut self, cx: &mut Context) -> Self::State {
    let _ = self.inner.poll_changes(cx);
    Box::new(self.inner.access_ref_collection())
  }
}

pub struct ReactiveManyOneRelationAsReactiveState<K, V, T> {
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, V, T> ReactiveState for ReactiveManyOneRelationAsReactiveState<K, V, T>
where
  K: CKey,
  V: CKey,
  T: ReactiveOneToManyRelationship<K, V>,
{
  type State = Box<dyn std::any::Any>;

  fn poll_current(&mut self, cx: &mut Context) -> Self::State {
    let _ = self.inner.poll_changes(cx);
    Box::new(self.inner.multi_access())
  }
}

pub struct ReactiveStateBoxAnyResult<T>(pub T);

impl<T> ReactiveState for ReactiveStateBoxAnyResult<T>
where
  T::State: 'static,
  T: ReactiveState,
{
  type State = Box<dyn std::any::Any>;

  fn poll_current(&mut self, cx: &mut Context) -> Self::State {
    Box::new(self.0.poll_current(cx))
  }
}
