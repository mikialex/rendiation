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

#[derive(Clone)]
pub struct FilterMapQueryChange<T, F> {
  pub base: T,
  pub mapper: F,
}

impl<F, V, V2, T> Query for FilterMapQueryChange<T, F>
where
  F: Fn(V) -> Option<V2> + Sync + Send + Clone + 'static,
  V2: CValue,
  T: Query<Value = ValueChange<V>>,
{
  type Key = T::Key;
  type Value = ValueChange<V2>;
  fn iter_key_value(&self) -> impl Iterator<Item = (T::Key, ValueChange<V2>)> + '_ {
    let checker = make_checker(self.mapper.clone());
    self
      .base
      .iter_key_value()
      .filter_map(move |(k, v)| (checker)(v).map(|v| (k, v)))
  }

  fn access(&self, key: &T::Key) -> Option<ValueChange<V2>> {
    let checker = make_checker(self.mapper.clone());
    let base = self.base.access(key)?;
    (checker)(base)
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
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let mapper = self.mapper.clone();
    let c = cx.resolve_cx().clone();
    let f = self
      .base
      .create_task(cx)
      .map(move |base| FilterMapQuery { base, mapper }.resolve(&c));

    avoid_huge_debug_symbols_by_boxing_future(f)
  }
}
