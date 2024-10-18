// use futures::{future::join, Future};

use crate::*;

pub trait ReactiveGeneralQuery {
  type Output;
  fn poll_query(&mut self, cx: &mut Context) -> Self::Output;
}

pub struct ReactiveQueryAsReactiveGeneralQuery<T> {
  pub inner: T,
}

impl<T> ReactiveGeneralQuery for ReactiveQueryAsReactiveGeneralQuery<T>
where
  T: ReactiveQuery,
{
  type Output = Box<dyn std::any::Any>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    let (_, v) = self.inner.poll_changes_dyn(cx);
    Box::new(v)
  }
}

pub struct ReactiveValueRefQueryAsReactiveGeneralQuery<T> {
  pub inner: T,
}

impl<T> ReactiveGeneralQuery for ReactiveValueRefQueryAsReactiveGeneralQuery<T>
where
  T: ReactiveValueRefQuery,
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

impl<T> ReactiveGeneralQuery for ReactiveManyOneRelationAsReactiveQuery<T>
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

impl<T> ReactiveGeneralQuery for ReactiveQueryBoxAnyResult<T>
where
  T::Output: 'static,
  T: ReactiveGeneralQuery,
{
  type Output = Box<dyn std::any::Any>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    Box::new(self.0.poll_query(cx))
  }
}
