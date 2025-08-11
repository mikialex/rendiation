use crate::*;

impl<T, F, V2> ReactiveQuery for FilterMapQuery<T, F>
where
  F: Fn(T::Value) -> Option<V2> + Clone + Send + Sync + 'static,
  T: ReactiveQuery,
  V2: CValue,
{
  type Key = T::Key;
  type Value = V2;
  type Compute = FilterMapQuery<T::Compute, F>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    let base = self.base.describe(cx);

    FilterMapQuery {
      base,
      mapper: self.mapper.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.base.request(request)
  }
}

impl<T, F, V2> QueryCompute for FilterMapQuery<T, F>
where
  F: Fn(T::Value) -> Option<V2> + Clone + Send + Sync + 'static,
  T: QueryCompute,
  V2: CValue,
{
  type Key = T::Key;
  type Value = V2;
  type Changes = FilterMapQueryChange<T::Changes, F>;
  type View = FilterMapQuery<T::View, F>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (d, v) = self.base.resolve(cx);

    let d = FilterMapQueryChange {
      base: d,
      mapper: self.mapper.clone(),
    };
    let v = v.filter_map(self.mapper.clone());

    (d, v)
  }
}

impl<T, F, V2> AsyncQueryCompute for FilterMapQuery<T, F>
where
  F: Fn(T::Value) -> Option<V2> + Clone + Send + Sync + 'static,
  T: AsyncQueryCompute,
  V2: CValue,
{
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    let mapper = self.mapper.clone();
    let c = cx.resolve_cx().clone();
    self
      .base
      .create_task(cx)
      .map(move |base| FilterMapQuery { base, mapper }.resolve(&c))
      .into_boxed_future()
  }
}
