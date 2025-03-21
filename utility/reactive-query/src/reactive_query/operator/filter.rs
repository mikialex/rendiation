use crate::*;

pub fn make_checker<V, V2>(
  checker: impl Fn(V) -> Option<V2> + Clone + Send + Sync + 'static,
) -> impl Fn(ValueChange<V>) -> Option<ValueChange<V2>> + Clone + Send + Sync + 'static {
  move |delta| {
    match delta {
      ValueChange::Delta(v, pre_v) => {
        let new_map = checker(v);
        let pre_map = pre_v.and_then(checker.clone());
        match (new_map, pre_map) {
          (Some(v), Some(pre_v)) => ValueChange::Delta(v, Some(pre_v)),
          (Some(v), None) => ValueChange::Delta(v, None),
          (None, Some(pre_v)) => ValueChange::Remove(pre_v),
          (None, None) => return None,
        }
        .into()
      }
      // the Remove variant maybe called many times for given k
      ValueChange::Remove(pre_v) => {
        let pre_map = checker(pre_v);
        match pre_map {
          Some(pre) => ValueChange::Remove(pre).into(),
          None => None,
        }
      }
    }
  }
}

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
