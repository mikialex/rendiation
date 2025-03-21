use crate::*;

impl<T, F, V2> ReactiveQuery for FilterMapQuery<T, F>
where
  F: Fn(T::Value) -> Option<V2> + Clone + Send + Sync + 'static,
  T: ReactiveQuery,
  V2: CValue,
{
  type Key = T::Key;
  type Value = V2;
  type Changes = impl Query<Key = Self::Key, Value = ValueChange<V2>>;
  type View = FilterMapQuery<T::View, F>;

  #[tracing::instrument(skip_all, name = "ReactiveKVFilter")]
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, v) = self.base.poll_changes(cx);

    let checker = make_checker(self.mapper.clone());
    let d = d.filter_map(checker);
    let v = v.filter_map(self.mapper.clone());

    (d, v)
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.base.request(request)
  }
}
