// use futures::{future::join, Future};

use crate::*;

pub trait ReactiveQuery {
  type Output;
  fn poll_query(&mut self, cx: &mut Context) -> Self::Output;
}

pub struct ReactiveCollectionAsReactiveQuery<T> {
  pub inner: T,
}

impl<T> ReactiveQuery for ReactiveCollectionAsReactiveQuery<T>
where
  T: ReactiveCollection,
{
  type Output = Box<dyn std::any::Any>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    let (_, v) = self.inner.poll_changes_dyn(cx);
    Box::new(v)
  }
}

pub struct ReactiveCollectionSelfContainedAsReactiveQuery<T> {
  pub inner: T,
}

impl<T> ReactiveQuery for ReactiveCollectionSelfContainedAsReactiveQuery<T>
where
  T: ReactiveCollectionSelfContained,
{
  type Output = Box<dyn std::any::Any>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    let (_, v) = self.inner.poll_changes_self_contained_dyn(cx);
    Box::new(v)
  }
}

pub struct ReactiveManyOneRelationAsReactiveQuery<T> {
  pub inner: T,
}

impl<T> ReactiveQuery for ReactiveManyOneRelationAsReactiveQuery<T>
where
  T: ReactiveOneToManyRelation,
{
  type Output = Box<dyn std::any::Any>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    let (_, _, m) = self.inner.poll_changes_with_inv_dyn(cx);
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
