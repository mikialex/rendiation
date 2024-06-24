// use futures::{future::join, Future};

use crate::*;

pub trait ReactiveQuery {
  type Output;
  fn poll_query(&mut self, cx: &mut Context) -> Self::Output;
}

// // another design direction:
// //
// pub trait ReactiveAsyncQuery {
//   type AsyncOutput;
//   type Output: Future<Output = Self::AsyncOutput> + 'static;

//   fn poll_async_query(&mut self, cx: &mut Context) -> Self::Output;

//   fn join_with<Other: ReactiveAsyncQuery>(self, other: Other) -> AsyncQueryJoin<Self, Other>
//   where
//     Self: Sized,
//   {
//     AsyncQueryJoin { a: self, b: other }
//   }
// }

// impl<T> ReactiveAsyncQuery for T
// where
//   T: ReactiveQuery<Output: futures::Future + 'static>,
// {
//   type AsyncOutput = <T::Output as Future>::Output;
//   type Output = T::Output;
//   fn poll_async_query(&mut self, cx: &mut Context) -> Self::Output {
//     self.poll_query(cx)
//   }
// }

// pub struct AsyncQueryJoin<A, B> {
//   a: A,
//   b: B,
// }

// impl<A, B> ReactiveAsyncQuery for AsyncQueryJoin<A, B>
// where
//   A: ReactiveAsyncQuery,
//   B: ReactiveAsyncQuery,
// {
//   type AsyncOutput = (A::AsyncOutput, B::AsyncOutput);
//   type Output = impl Future<Output = Self::AsyncOutput> + 'static;

//   fn poll_async_query(&mut self, cx: &mut Context) -> Self::Output {
//     let a = self.a.poll_async_query(cx);
//     let b = self.b.poll_async_query(cx);
//     join(a, b)
//   }
// }

// pub trait ReactiveCollectiveUpdate<K: CKey, V: CValue> {
//   type Current: VirtualCollection<K, V>;
//   type Delta: VirtualCollection<K, ValueChange<V>>;
// }

// pub trait ReactiveCollectionImproved<K: CKey, V: CValue>:
//   ReactiveAsyncQuery<AsyncQuery: ReactiveCollectiveUpdate<K, V>>
// {
// }

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
