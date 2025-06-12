// use futures::{future::join, Future};

use crate::*;

/// the difference between this and the Stream is that:
/// - the output will always available when the query is polled instead of return pending or termination
///   - of course, the pending or termination info could be included in the output depend on the user's demands
/// - self parameter is not required to be pinned for sake of simplicity
pub trait ReactiveGeneralQuery {
  type Output;
  fn poll_query(&mut self, cx: &mut Context) -> Self::Output;
}

pub struct IntoBoxedAnyReactiveGeneralQuery<T>(pub T);

impl<T> ReactiveGeneralQuery for IntoBoxedAnyReactiveGeneralQuery<T>
where
  T: ReactiveGeneralQuery,
  T::Output: 'static,
{
  type Output = Box<dyn std::any::Any>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    let r = self.0.poll_query(cx);
    Box::new(r)
  }
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
    let (_, v) = self.inner.describe_dyn(cx).resolve_kept();
    Box::new(v.into_boxed())
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
    let (_, m) = self.inner.describe_with_inv_dyn(cx).resolve_kept();
    Box::new(Box::new(m) as BoxedDynMultiQuery<T::One, T::Many>)
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
