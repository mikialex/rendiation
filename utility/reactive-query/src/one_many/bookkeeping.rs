use crate::*;

pub struct OneToManyRefHashBookKeeping<T, K, V> {
  pub upstream: T,
  pub mapping: Arc<RwLock<FastHashMap<V, FastHashSet<K>>>>,
}

impl<T> ReactiveQuery for OneToManyRefHashBookKeeping<T, T::Key, T::Value>
where
  T: ReactiveQuery,
  T::Value: CKey,
{
  type Key = T::Key;
  type Value = T::Value;

  type Compute = OneToManyRefHashBookKeeping<T::Compute, T::Key, T::Value>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    OneToManyRefHashBookKeeping {
      upstream: self.upstream.describe(cx),
      mapping: self.mapping.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.upstream.request(request);
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => self.mapping.write().shrink_to_fit(),
    }
  }
}

impl<T> QueryCompute for OneToManyRefHashBookKeeping<T, T::Key, T::Value>
where
  T: QueryCompute,
  T::Value: CKey,
{
  type Key = T::Key;
  type Value = T::Value;

  type Changes = T::Changes;
  type View =
    QueryAndMultiQuery<T::View, LockReadGuardHolder<FastHashMap<T::Value, FastHashSet<T::Key>>>>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (r, r_view) = self.upstream.resolve(cx);

    {
      let mut mapping = self.mapping.write();
      bookkeeping_hash_relation(&mut mapping, &r);
    }

    let v = QueryAndMultiQuery {
      query: r_view,
      multi_query: self.mapping.make_read_holder(),
    };

    (r, v)
  }
}
impl<T> AsyncQueryCompute for OneToManyRefHashBookKeeping<T, T::Key, T::Value>
where
  T: AsyncQueryCompute,
  T::Value: CKey,
{
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    let mapping = self.mapping.clone();
    let upstream = self.upstream.create_task(cx);
    cx.then_spawn_compute(upstream, |upstream| OneToManyRefHashBookKeeping {
      upstream,
      mapping,
    })
    .into_boxed_future()
  }
}

pub struct OneToManyRefDenseBookKeeping<T> {
  pub upstream: T,
  pub mapping: Arc<RwLock<DenseIndexMapping>>,
}

#[derive(Clone)]
pub struct OneToManyRefDenseBookKeepingCurrentView<T> {
  upstream: T,
  mapping: LockReadGuardHolder<DenseIndexMapping>,
}

impl<T> Query for OneToManyRefDenseBookKeepingCurrentView<T>
where
  T: Query,
{
  type Key = T::Key;
  type Value = T::Value;
  fn access(&self, m: &T::Key) -> Option<T::Value> {
    self.upstream.access(m)
  }

  fn iter_key_value(&self) -> impl Iterator<Item = (T::Key, T::Value)> + '_ {
    self.upstream.iter_key_value()
  }
}

impl<T> MultiQuery for OneToManyRefDenseBookKeepingCurrentView<T>
where
  T: Query,
  T::Key: CKey + LinearIdentification,
  T::Value: CKey + LinearIdentification,
{
  type Key = T::Value;
  type Value = T::Key;
  fn iter_keys(&self) -> impl Iterator<Item = T::Value> + '_ {
    self.mapping.iter_keys().map(T::Value::from_alloc_index)
  }

  fn access_multi(&self, o: &T::Value) -> Option<impl Iterator<Item = T::Key> + '_> {
    let i = &o.alloc_index();
    let i = unsafe { std::mem::transmute(i) }; // todo fix
    let iter = self.mapping.access_multi(i)?;
    iter.map(T::Key::from_alloc_index).into()
  }
}

impl<T> ReactiveQuery for OneToManyRefDenseBookKeeping<T>
where
  T: ReactiveQuery,
  T::Value: LinearIdentification + CKey,
  T::Key: LinearIdentification + CKey,
{
  type Key = T::Key;
  type Value = T::Value;
  type Compute = OneToManyRefDenseBookKeeping<T::Compute>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    OneToManyRefDenseBookKeeping {
      upstream: self.upstream.describe(cx),
      mapping: self.mapping.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.upstream.request(request);
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => {
        let mut mapping = self.mapping.write();
        mapping.shrink_to_fit();
      }
    }
  }
}

impl<T> QueryCompute for OneToManyRefDenseBookKeeping<T>
where
  T: QueryCompute,
  T::Value: LinearIdentification + CKey,
  T::Key: LinearIdentification + CKey,
{
  type Key = T::Key;
  type Value = T::Value;

  type Changes = T::Changes;
  type View = OneToManyRefDenseBookKeepingCurrentView<T::View>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (r, r_view) = self.upstream.resolve(cx);

    {
      let mut mapping = self.mapping.write();
      let mapping: &mut DenseIndexMapping = &mut mapping;
      bookkeeping_dense_index_relation(mapping, &r);
    }

    let v = OneToManyRefDenseBookKeepingCurrentView {
      upstream: r_view,
      mapping: self.mapping.make_read_holder(),
    };

    (r, v)
  }
}

impl<T> AsyncQueryCompute for OneToManyRefDenseBookKeeping<T>
where
  T: AsyncQueryCompute,
  T::Value: LinearIdentification + CKey,
  T::Key: LinearIdentification + CKey,
{
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    let mapping = self.mapping.clone();
    let upstream = self.upstream.create_task(cx);
    cx.then_spawn_compute(upstream, |upstream| OneToManyRefDenseBookKeeping {
      upstream,
      mapping,
    })
    .into_boxed_future()
  }
}
