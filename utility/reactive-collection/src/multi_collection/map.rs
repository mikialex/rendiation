use crate::*;

pub struct ReactiveKVMapRelation<T, F, F1, F2, K, V> {
  pub inner: T,
  pub map: F,
  pub f1: F1,
  pub f2: F2,
  pub phantom: PhantomData<(K, V)>,
}

impl<T, F, F1, F2, K, V, V2> ReactiveCollection<K, V2> for ReactiveKVMapRelation<T, F, F1, F2, K, V>
where
  V: CKey,
  K: CKey,
  V2: CKey,
  F: Fn(&K, V) -> V2 + Copy + Send + Sync + 'static,
  F1: Fn(V) -> V2 + Copy + Send + Sync + 'static,
  F2: Fn(V2) -> V + Copy + Send + Sync + 'static,
  T: ReactiveOneToManyRelation<V, K>,
{
  type Changes = impl VirtualCollection<K, ValueChange<V2>>;
  type View = impl VirtualCollection<K, V2> + VirtualMultiCollection<V2, K>;
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn poll_changes(&self, cx: &mut Context) -> Self::Task {
    let f = self.inner.poll_changes(cx);
    let map = self.map;
    let f1 = self.f1;
    let f2 = self.f2;
    async move {
      let (d, v) = f.await;
      let d = d.map(move |k, v| v.map(|v| map(k, v)));

      let v_inv = v.clone().multi_key_dual_map(f1, f2);
      let v = v.map(map);
      let v = OneManyRelationDualAccess {
        many_access_one: v,
        one_access_many: v_inv,
      };

      (d, v)
    }
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

pub struct ReactiveKeyDualMapRelation<F1, F2, T, K, V> {
  pub f1: F1,
  pub f2: F2,
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<F1, F2, T, K, K2, V> ReactiveCollection<K2, V> for ReactiveKeyDualMapRelation<F1, F2, T, K, V>
where
  K: CKey,
  K2: CKey,
  V: CKey,
  F1: Fn(K) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> K + Copy + Send + Sync + 'static,
  T: ReactiveOneToManyRelation<V, K>,
{
  type Changes = impl VirtualCollection<K2, ValueChange<V>>;
  type View = impl VirtualCollection<K2, V> + VirtualMultiCollection<V, K2>;
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn poll_changes(&self, cx: &mut Context) -> Self::Task {
    let f1 = self.f1;
    let f2 = self.f2;
    let f = self.inner.poll_changes(cx);

    async move {
      let (d, v) = f.await;
      let d = d.key_dual_map(f1, f2);
      let f1_ = f1;
      let v_inv = v.clone().multi_map(move |_, v| f1_(v));
      let v = v.key_dual_map(f1, f2);
      let v = OneManyRelationDualAccess {
        many_access_one: v,
        one_access_many: v_inv,
      };
      (d, v)
    }
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}
