use crate::*;

pub struct ReactiveKVMapRelation<T, F, F1, F2> {
  pub inner: T,
  pub map: F,
  pub f1: F1,
  pub f2: F2,
}

impl<T, F, F1, F2, V2> QueryCompute for ReactiveKVMapRelation<T, F, F1, F2>
where
  V2: CKey,
  F: Fn(&T::Many, T::One) -> V2 + Copy + Send + Sync + 'static,
  F1: Fn(T::One) -> V2 + Copy + Send + Sync + 'static,
  F2: Fn(V2) -> T::One + Copy + Send + Sync + 'static,
  T: ReactiveOneToManyRelationCompute,
{
  type Key = T::Many;
  type Value = V2;

  type Changes = MappedQuery<T::Changes, ValueChangeMapper<F>>;
  type View = OneManyRelationDualAccess<
    MappedQuery<T::View, F>,
    KeyDualMappedQuery<T::View, F1, AutoSomeFnResult<F2>>,
  >;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (d, v) = self.inner.resolve(cx);
    let map = self.map;
    let d = d.map(ValueChangeMapper(map));

    let v_inv = v.clone().multi_key_dual_map(self.f1, self.f2);
    let v = v.map(self.map);
    let v = OneManyRelationDualAccess {
      many_access_one: v,
      one_access_many: v_inv,
    };

    (d, v)
  }
}

impl<T, F, F1, F2, V2> AsyncQueryCompute for ReactiveKVMapRelation<T, F, F1, F2>
where
  V2: CKey,
  F: Fn(&T::Many, T::One) -> V2 + Copy + Send + Sync + 'static,
  F1: Fn(T::One) -> V2 + Copy + Send + Sync + 'static,
  F2: Fn(V2) -> T::One + Copy + Send + Sync + 'static,
  T: ReactiveOneToManyRelationCompute + AsyncQueryCompute,
{
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    let map = self.map;
    let f1 = self.f1;
    let f2 = self.f2;
    let c = cx.resolve_cx().clone();
    self
      .inner
      .create_task(cx)
      .map(move |inner| ReactiveKVMapRelation { inner, map, f1, f2 }.resolve(&c))
      .into_boxed_future()
  }
}

impl<T, F, F1, F2, V2> ReactiveQuery for ReactiveKVMapRelation<T, F, F1, F2>
where
  V2: CKey,
  F: Fn(&T::Many, T::One) -> V2 + Copy + Send + Sync + 'static,
  F1: Fn(T::One) -> V2 + Copy + Send + Sync + 'static,
  F2: Fn(V2) -> T::One + Copy + Send + Sync + 'static,
  T: ReactiveOneToManyRelation,
{
  type Key = T::Many;
  type Value = V2;

  type Compute = ReactiveKVMapRelation<T::Compute, F, F1, F2>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    ReactiveKVMapRelation {
      inner: self.inner.describe(cx),
      map: self.map,
      f1: self.f1,
      f2: self.f2,
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.inner.request(request)
  }
}

pub struct ReactiveKeyDualMapRelation<F1, F2, T> {
  pub f1: F1,
  pub f2: F2,
  pub inner: T,
}

impl<F1, F2, T, K2> ReactiveQuery for ReactiveKeyDualMapRelation<F1, F2, T>
where
  K2: CKey,
  F1: Fn(T::Many) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> T::Many + Copy + Send + Sync + 'static,
  T: ReactiveOneToManyRelation,
{
  type Key = K2;
  type Value = T::One;

  type Compute = ReactiveKeyDualMapRelation<F1, F2, T::Compute>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    ReactiveKeyDualMapRelation {
      inner: self.inner.describe(cx),
      f1: self.f1,
      f2: self.f2,
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.inner.request(request)
  }
}

impl<F1, F2, T, K2> QueryCompute for ReactiveKeyDualMapRelation<F1, F2, T>
where
  K2: CKey,
  F1: Fn(T::Many) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> T::Many + Copy + Send + Sync + 'static,
  T: ReactiveOneToManyRelationCompute,
{
  type Key = K2;
  type Value = T::One;
  type Changes = KeyDualMappedQuery<T::Changes, F1, AutoSomeFnResult<F2>>;
  type View = OneManyRelationDualAccess<
    KeyDualMappedQuery<T::View, F1, AutoSomeFnResult<F2>>,
    MappedValueQuery<T::View, F1>,
  >;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (d, v) = self.inner.resolve(cx);
    let d = d.key_dual_map(self.f1, self.f2);
    let v_inv = v.clone().multi_map(self.f1);
    let v = v.key_dual_map(self.f1, self.f2);
    let v = OneManyRelationDualAccess {
      many_access_one: v,
      one_access_many: v_inv,
    };
    (d, v)
  }
}

impl<F1, F2, T, K2> AsyncQueryCompute for ReactiveKeyDualMapRelation<F1, F2, T>
where
  K2: CKey,
  F1: Fn(T::Many) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> T::Many + Copy + Send + Sync + 'static,
  T: ReactiveOneToManyRelationCompute + AsyncQueryCompute,
{
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    let f1 = self.f1;
    let f2 = self.f2;
    let c = cx.resolve_cx().clone();
    self
      .inner
      .create_task(cx)
      .map(move |inner| ReactiveKeyDualMapRelation { inner, f1, f2 }.resolve(&c))
      .into_boxed_future()
  }
}
