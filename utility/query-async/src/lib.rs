#![feature(impl_trait_in_assoc_type)]

use std::{future::Future, sync::Arc};

use futures::FutureExt;
use query::*;

pub trait AsyncQuery {
  type Key: CKey;
  type Value: CValue;

  type Query;
  type Task: Future<Output = Self::Query>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task;
}

pub struct AsyncQueryCtx;

impl AsyncQueryCtx {
  pub fn spawn_task<R>(
    &mut self,
    f: impl Fn() -> R + 'static,
  ) -> impl Future<Output = R> + 'static {
    // todo, use some thread pool impl
    async move { f() }
  }
}

/// convert a query into async query by spawning it's materialization computation into thread pool
pub struct MaterializeQueryAsync<T>(pub T);

impl<T: Query + 'static> AsyncQuery for MaterializeQueryAsync<T> {
  type Key = T::Key;
  type Value = T::Value;

  type Query = Arc<QueryMaterialized<Self::Key, Self::Value>>;
  type Task = impl Future<Output = Self::Query>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let task = self.0.clone();
    cx.spawn_task(move || task.materialize())
  }
}

impl<F, T> AsyncQuery for MappedQuery<T, F>
where
  F: Clone,
  T: AsyncQuery,
{
  type Key = T::Key;
  type Value = T::Value;

  type Query = MappedQuery<T::Query, F>;
  type Task = impl Future<Output = Self::Query>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let mapper = self.mapper.clone();
    self
      .base
      .create_task(cx)
      .map(|base| MappedQuery { mapper, base })
  }
}

impl<F, V2, T> AsyncQuery for FilterMapQuery<T, F>
where
  F: Fn(T::Value) -> Option<V2> + Sync + Send + Clone + 'static,
  V2: CValue,
  T: AsyncQuery,
{
  type Key = T::Key;
  type Value = V2;

  type Query = FilterMapQuery<T::Query, F>;
  type Task = impl Future<Output = Self::Query>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let mapper = self.mapper.clone();
    self
      .base
      .create_task(cx)
      .map(|base| FilterMapQuery { mapper, base })
  }
}

impl<K2, F1, F2, T> AsyncQuery for KeyDualMappedQuery<F1, F2, T>
where
  K2: CKey,
  F1: Fn(T::Key) -> K2 + Clone + Send + Sync + 'static,
  F2: Fn(K2) -> Option<T::Key> + Clone + Send + Sync + 'static,
  T: AsyncQuery,
{
  type Key = K2;
  type Value = T::Value;

  type Query = KeyDualMappedQuery<F1, F2, T::Query>;
  type Task = impl Future<Output = Self::Query>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let f1 = self.f1.clone();
    let f2 = self.f2.clone();

    self
      .base
      .create_task(cx)
      .map(|base| KeyDualMappedQuery { base, f1, f2 })
  }
}

impl<A: AsyncQuery, B: AsyncQuery> AsyncQuery for CrossJoinQuery<A, B> {
  type Key = (A::Key, B::Key);
  type Value = (A::Value, B::Value);

  type Query = CrossJoinQuery<A::Query, B::Query>;
  type Task = impl Future<Output = Self::Query>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let a = self.a.create_task(cx);
    let b = self.b.create_task(cx);

    futures::future::join(a, b).map(|(a, b)| CrossJoinQuery { a, b })
  }
}

impl<A, B, F, O> AsyncQuery for UnionQuery<A, B, F>
where
  A: AsyncQuery,
  B: AsyncQuery<Key = A::Key>,
  F: Fn((Option<A::Value>, Option<B::Value>)) -> Option<O> + Send + Sync + Clone + 'static,

  O: CValue,
{
  type Key = A::Key;
  type Value = O;

  type Query = UnionQuery<A::Query, B::Query, F>;
  type Task = impl Future<Output = Self::Query>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let a = self.a.create_task(cx);
    let b = self.b.create_task(cx);

    let f = self.f.clone();

    futures::future::join(a, b).map(move |(a, b)| UnionQuery { a, b, f })
  }
}
