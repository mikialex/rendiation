use crate::*;

pub fn make_checker<V, V2>(
  checker: impl Fn(V) -> Option<V2> + Copy + Send + Sync + 'static,
) -> impl Fn(ValueChange<V>) -> Option<ValueChange<V2>> + Copy + Send + Sync + 'static {
  move |delta| {
    match delta {
      ValueChange::Delta(v, pre_v) => {
        let new_map = checker(v);
        let pre_map = pre_v.and_then(checker);
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

pub struct ReactiveKVFilter<T, F, K, V> {
  pub inner: T,
  pub checker: F,
  pub k: PhantomData<(K, V)>,
}

impl<T, F, K, V, V2> ReactiveCollection<K, V2> for ReactiveKVFilter<T, F, K, V>
where
  F: Fn(V) -> Option<V2> + Copy + Send + Sync + 'static,
  T: ReactiveCollection<K, V> + Sync,
  K: CKey,
  V: CValue,
  V2: CValue,
{
  type Changes = impl VirtualCollection<K, ValueChange<V2>>;
  type View = impl VirtualCollection<K, V2>;

  #[tracing::instrument(skip_all, name = "ReactiveKVFilter")]
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, v) = self.inner.poll_changes(cx);

    let checker = make_checker(self.checker);
    let d = d.filter_map(checker);
    let v = v.filter_map(self.checker);

    (d, v)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}
