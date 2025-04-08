// use futures::{future::join, Future};

use crate::*;

pub trait ReactiveGeneralQuery {
  type Output;
  fn poll_query(
    &mut self,
    cx: &mut Context,
    acx: &mut AsyncQueryCtx,
  ) -> Pin<Box<dyn Future<Output = Self::Output>>>;
}

pub struct ReactiveQueryAsReactiveGeneralQuery<T> {
  pub inner: T,
}

impl<T> ReactiveGeneralQuery for ReactiveQueryAsReactiveGeneralQuery<T>
where
  T: ReactiveQuery,
{
  type Output = Box<dyn std::any::Any>;

  fn poll_query(
    &mut self,
    cx: &mut Context,
    acx: &mut AsyncQueryCtx,
  ) -> Pin<Box<dyn Future<Output = Self::Output>>> {
    let f = self
      .inner
      .describe_dyn(cx)
      .create_task(acx)
      .map(|(_, v)| Box::new(v.into_boxed()) as Box<dyn std::any::Any>);
    Box::pin(f)
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

  fn poll_query(
    &mut self,
    cx: &mut Context,
    acx: &mut AsyncQueryCtx,
  ) -> Pin<Box<dyn Future<Output = Self::Output>>> {
    let f = self
      .inner
      .describe_dyn(cx)
      .create_task(acx)
      .map(|(_, v)| Box::new(v) as Box<dyn std::any::Any>);
    Box::pin(f)
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

  fn poll_query(
    &mut self,
    cx: &mut Context,
    acx: &mut AsyncQueryCtx,
  ) -> Pin<Box<dyn Future<Output = Self::Output>>> {
    let f = self
      .inner
      .describe_with_inv_dyn(cx)
      .create_task(acx)
      .map(|(_, v)| {
        Box::new(Box::new(v) as BoxedDynMultiQuery<T::One, T::Many>) as Box<dyn std::any::Any>
      });
    Box::pin(f)
  }
}

pub struct ReactiveQueryBoxAnyResult<T>(pub T);

impl<T> ReactiveGeneralQuery for ReactiveQueryBoxAnyResult<T>
where
  T::Output: 'static,
  T: ReactiveGeneralQuery,
{
  type Output = Box<dyn std::any::Any>;

  fn poll_query(
    &mut self,
    cx: &mut Context,
    acx: &mut AsyncQueryCtx,
  ) -> Pin<Box<dyn Future<Output = Self::Output>>> {
    let f = self
      .0
      .poll_query(cx, acx)
      .map(|v| Box::new(v) as Box<dyn std::any::Any>);

    Box::pin(f)
  }
}
