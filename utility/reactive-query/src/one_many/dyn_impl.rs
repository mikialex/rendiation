use crate::*;

pub type BoxedDynReactiveOneToManyRelation<O, M> =
  Box<dyn DynReactiveOneToManyRelation<One = O, Many = M>>;

pub trait DynReactiveOneToManyRelation: Send + Sync {
  type One: CKey;
  type Many: CKey;
  /// we could return a single trait object that cover both access and inverse access
  /// but for simplicity we just return two trait objects as these two trait both impl clone.
  fn describe_with_inv_dyn(
    &self,
    cx: &mut Context,
  ) -> BoxedDynOneToManyQueryCompute<Self::Many, Self::One>;

  fn extra_request_dyn(&mut self, request: &mut ReactiveQueryRequest);
}

impl<T> DynReactiveOneToManyRelation for T
where
  T: ReactiveOneToManyRelation,
{
  type One = T::One;
  type Many = T::Many;
  fn describe_with_inv_dyn(
    &self,
    cx: &mut Context,
  ) -> BoxedDynOneToManyQueryCompute<Self::Many, Self::One> {
    Box::new(self.describe(cx))
  }

  fn extra_request_dyn(&mut self, request: &mut ReactiveQueryRequest) {
    self.request(request)
  }
}

impl<O, M> ReactiveQuery for BoxedDynReactiveOneToManyRelation<O, M>
where
  O: CKey,
  M: CKey,
{
  type Key = M;
  type Value = O;
  type Compute = BoxedDynOneToManyQueryCompute<M, O>;
  fn describe(&self, cx: &mut Context) -> Self::Compute {
    self.deref().describe_with_inv_dyn(cx)
  }
  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.deref_mut().extra_request_dyn(request)
  }
}

pub type BoxedDynOneToManyQueryCompute<M, O> = Box<dyn DynOneToManyQueryCompute<One = O, Many = M>>;
pub trait DynOneToManyQueryCompute: Sync + Send + 'static {
  type One: CKey;
  type Many: CKey;
  fn resolve_one_many_dyn(
    &mut self,
    cx: &QueryResolveCtx,
  ) -> DynOneToManyQueryComputePoll<Self::Many, Self::One>;
  fn create_one_many_task_dyn(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> Pin<
    Box<dyn Send + Sync + Future<Output = DynOneToManyQueryComputePoll<Self::Many, Self::One>>>,
  >;
}

impl<T> DynOneToManyQueryCompute for T
where
  T: ReactiveOneToManyRelationCompute,
{
  type Many = T::Key;
  type One = T::Value;
  fn resolve_one_many_dyn(
    &mut self,
    cx: &QueryResolveCtx,
  ) -> DynOneToManyQueryComputePoll<Self::Many, Self::One> {
    let (d, v) = self.resolve(cx);
    let v = OneManyRelationDualAccess {
      many_access_one: Box::new(v.clone()) as BoxedDynQuery<Self::Many, Self::One>,
      one_access_many: Box::new(v) as BoxedDynMultiQuery<Self::One, Self::Many>,
    };
    (Box::new(d), v)
  }
  fn create_one_many_task_dyn(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> Pin<
    Box<dyn Send + Sync + Future<Output = DynOneToManyQueryComputePoll<Self::Many, Self::One>>>,
  > {
    #[cfg(not(debug_assertions))]
    {
      let c = cx.resolve_cx().clone();
      self
        .create_task(cx)
        .map(move |mut r| r.resolve_one_many_dyn(&c))
        .into_boxed_future()
    }

    // disable async support in debug mode, to avoid huge debug symbol
    #[cfg(debug_assertions)]
    {
      std::future::ready(self.resolve_one_many_dyn(cx.resolve_cx())).into_boxed_future()
    }
  }
}

impl<M: CKey, O: CKey> QueryCompute for BoxedDynOneToManyQueryCompute<M, O> {
  type Key = M;
  type Value = O;
  type Changes = BoxedDynQuery<M, ValueChange<O>>;
  type View = OneManyRelationDualAccess<BoxedDynQuery<M, O>, BoxedDynMultiQuery<O, M>>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    self.deref_mut().resolve_one_many_dyn(cx)
  }
}
impl<M: CKey, O: CKey> AsyncQueryCompute for BoxedDynOneToManyQueryCompute<M, O> {
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    self.deref_mut().create_one_many_task_dyn(cx)
  }
}

type DynOneToManyQueryComputePoll<M, O> = (
  BoxedDynQuery<M, ValueChange<O>>,
  OneManyRelationDualAccess<BoxedDynQuery<M, O>, BoxedDynMultiQuery<O, M>>,
);

#[derive(Clone)]
pub struct OneManyRelationDualAccess<T, IT> {
  pub many_access_one: T,
  pub one_access_many: IT,
}

impl<O, M, T, IT> Query for OneManyRelationDualAccess<T, IT>
where
  O: CKey,
  M: CKey,
  T: Query<Key = M, Value = O>,
  IT: MultiQuery<Key = O, Value = M>,
{
  type Key = M;
  type Value = O;
  fn iter_key_value(&self) -> impl Iterator<Item = (M, O)> + '_ {
    self.many_access_one.iter_key_value()
  }

  fn access(&self, key: &M) -> Option<O> {
    self.many_access_one.access(key)
  }
}

impl<O, M, T, IT> MultiQuery for OneManyRelationDualAccess<T, IT>
where
  O: CKey,
  M: CKey,
  T: Query<Key = M, Value = O>,
  IT: MultiQuery<Key = O, Value = M>,
{
  type Key = O;
  type Value = M;
  fn iter_keys(&self) -> impl Iterator<Item = O> + '_ {
    self.one_access_many.iter_keys()
  }

  fn access_multi(&self, key: &O) -> Option<impl Iterator<Item = M> + '_> {
    self.one_access_many.access_multi(key)
  }
}
