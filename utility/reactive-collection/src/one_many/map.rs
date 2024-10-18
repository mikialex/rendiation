use crate::*;

pub struct ReactiveKVMapRelation<T, F, F1, F2> {
  pub inner: T,
  pub map: F,
  pub f1: F1,
  pub f2: F2,
}

impl<T, F, F1, F2, V2> ReactiveCollection for ReactiveKVMapRelation<T, F, F1, F2>
where
  V2: CKey,
  F: Fn(&T::Many, T::One) -> V2 + Copy + Send + Sync + 'static,
  F1: Fn(T::One) -> V2 + Copy + Send + Sync + 'static,
  F2: Fn(V2) -> T::One + Copy + Send + Sync + 'static,
  T: ReactiveOneToManyRelation,
{
  type Key = T::Many;
  type Value = V2;
  type Changes = impl VirtualCollection<Key = T::Many, Value = ValueChange<V2>>;
  type View = impl VirtualCollection<Key = T::Many, Value = V2>
    + VirtualMultiCollection<Key = V2, Value = T::Many>;

  #[tracing::instrument(skip_all, name = "ReactiveKVMap")]
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, v) = self.inner.poll_changes(cx);
    let map = self.map;
    let d = d.map(move |k, v| v.map(|v| map(k, v)));

    let v_inv = v.clone().multi_key_dual_map(self.f1, self.f2);
    let v = v.map(self.map);
    let v = OneManyRelationDualAccess {
      many_access_one: v,
      one_access_many: v_inv,
    };

    (d, v)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

pub struct ReactiveKeyDualMapRelation<F1, F2, T> {
  pub f1: F1,
  pub f2: F2,
  pub inner: T,
}

impl<F1, F2, T, K2> ReactiveCollection for ReactiveKeyDualMapRelation<F1, F2, T>
where
  K2: CKey,
  F1: Fn(T::Many) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> T::Many + Copy + Send + Sync + 'static,
  T: ReactiveOneToManyRelation,
{
  type Key = K2;
  type Value = T::One;
  type Changes = impl VirtualCollection<Key = K2, Value = ValueChange<T::One>>;
  type View = impl VirtualCollection<Key = K2, Value = T::One>
    + VirtualMultiCollection<Key = T::One, Value = K2>;

  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, v) = self.inner.poll_changes(cx);
    let d = d.key_dual_map(self.f1, self.f2);
    let f1_ = self.f1;
    let v_inv = v.clone().multi_map(move |_, v| f1_(v));
    let v = v.key_dual_map(self.f1, self.f2);
    let v = OneManyRelationDualAccess {
      many_access_one: v,
      one_access_many: v_inv,
    };
    (d, v)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}
