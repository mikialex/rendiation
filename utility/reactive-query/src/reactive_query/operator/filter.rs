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

  fn poll_changes(&self, cx: &mut Context) -> Self::Compute {
    let base = self.base.poll_changes(cx);

    FilterMapQuery {
      base,
      mapper: self.mapper.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.base.request(request)
  }
}

impl<T, F, V2> ReactiveQueryCompute for FilterMapQuery<T, F>
where
  F: Fn(T::Value) -> Option<V2> + Clone + Send + Sync + 'static,
  T: ReactiveQueryCompute,
  V2: CValue,
{
  type Key = T::Key;
  type Value = V2;
  type Changes = impl Query<Key = Self::Key, Value = ValueChange<V2>> + 'static;
  type View = FilterMapQuery<T::View, F>;

  fn resolve(self) -> (Self::Changes, Self::View) {
    let (d, v) = self.base.resolve();

    let checker = make_checker(self.mapper.clone());
    let d = d.filter_map(checker);
    let v = v.filter_map(self.mapper.clone());

    (d, v)
  }
}
