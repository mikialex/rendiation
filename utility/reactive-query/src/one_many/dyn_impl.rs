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
    self.describe_with_inv_dyn(cx)
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
  ) -> Box<
    dyn Send + Sync + Unpin + Future<Output = DynOneToManyQueryComputePoll<Self::Many, Self::One>>,
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
    (Box::new(d), Box::new(v.clone()), Box::new(v))
  }
  fn create_one_many_task_dyn(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> Box<
    dyn Send + Sync + Unpin + Future<Output = DynOneToManyQueryComputePoll<Self::Many, Self::One>>,
  > {
    // let c = cx.resolve_cx().clone();
    // Box::new(Box::pin(
    //   self.create_task(cx).map(move |mut r| r.resolve_dyn(&c)),
    // ))
    let f = std::future::ready(self.resolve_one_many_dyn(cx.resolve_cx()));
    Box::new(f)
  }
}

impl<M: CKey, O: CKey> QueryCompute for BoxedDynOneToManyQueryCompute<M, O> {
  type Key = M;
  type Value = O;
  type Changes = BoxedDynQuery<M, ValueChange<O>>;
  type View = BoxedDynQuery<M, O>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (d, v, _) = self.deref_mut().resolve_one_many_dyn(cx);
    (d, v)
  }
}
impl<M: CKey, O: CKey> AsyncQueryCompute for BoxedDynOneToManyQueryCompute<M, O> {
  type Task = impl Future<Output = (Self::Changes, Self::View)> + 'static;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    self.create_task_dyn(cx)
  }
}

type DynOneToManyQueryComputePoll<M, O> = (
  BoxedDynQuery<M, ValueChange<O>>,
  BoxedDynQuery<M, O>,
  BoxedDynMultiQuery<O, M>,
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
