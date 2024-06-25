// use futures::{future::join, Future};

use crate::*;

pub trait ReactiveQuery {
  type Output;
  fn poll_query(&mut self, cx: &mut Context) -> Self::Output;
}

pub struct ReactiveCollectionAsReactiveQuery<K, V, T> {
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, V, T> ReactiveQuery for ReactiveCollectionAsReactiveQuery<K, V, T>
where
  K: CKey,
  V: CValue,
  T: ReactiveCollection<K, V>,
{
  type Output = Box<dyn std::any::Any>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    let (_, v) = self.inner.poll_changes_dyn(cx);
    Box::new(v)
  }
}

pub struct ReactiveCollectionSelfContainedAsReactiveQuery<K, V, T> {
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, V, T> ReactiveQuery for ReactiveCollectionSelfContainedAsReactiveQuery<K, V, T>
where
  K: CKey,
  V: CValue,
  T: ReactiveCollectionSelfContained<K, V>,
{
  type Output = Box<dyn std::any::Any>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    let (_, v) = self.inner.poll_changes_dyn(cx);
    Box::new(v)
  }
}

pub struct ReactiveManyOneRelationAsReactiveQuery<K, V, T> {
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, V, T> ReactiveQuery for ReactiveManyOneRelationAsReactiveQuery<K, V, T>
where
  K: CKey,
  V: CKey,
  T: ReactiveOneToManyRelation<K, V>,
{
  type Output = Box<dyn std::any::Any>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    let (_, m) = self.inner.poll_changes_dyn(cx);
    Box::new(m)
  }
}

pub struct ReactiveQueryBoxAnyResult<T>(pub T);

impl<T> ReactiveQuery for ReactiveQueryBoxAnyResult<T>
where
  T::Output: 'static,
  T: ReactiveQuery,
{
  type Output = Box<dyn std::any::Any>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    Box::new(self.0.poll_query(cx))
  }
}
