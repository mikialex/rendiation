use crate::*;

pub type BoxedDynReactiveOneToManyRelation<O, M> =
  Box<dyn DynReactiveOneToManyRelation<One = O, Many = M>>;

pub type BoxedDynReactiveOneToManyRelationPoll<O, M> = (
  BoxedDynQuery<M, ValueChange<O>>,
  BoxedDynQuery<M, O>,
  BoxedDynMultiQuery<O, M>,
);

pub trait DynReactiveOneToManyRelation: Send + Sync {
  type One: CKey;
  type Many: CKey;
  /// we could return a single trait object that cover both access and inverse access
  /// but for simplicity we just return two trait objects as these two trait both impl clone.
  fn poll_changes_with_inv_dyn(
    &self,
    cx: &mut Context,
  ) -> BoxedDynReactiveOneToManyRelationPoll<Self::One, Self::Many>;

  fn extra_request_dyn(&mut self, request: &mut ReactiveQueryRequest);
}

impl<T> DynReactiveOneToManyRelation for T
where
  T: ReactiveOneToManyRelation,
{
  type One = T::One;
  type Many = T::Many;
  fn poll_changes_with_inv_dyn(
    &self,
    cx: &mut Context,
  ) -> BoxedDynReactiveOneToManyRelationPoll<Self::One, Self::Many> {
    let (d, v) = self.describe(cx).resolve();
    (Box::new(d), Box::new(v.clone()), Box::new(v))
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
  type Compute = (
    impl Query<Key = M, Value = ValueChange<O>>,
    impl Query<Key = M, Value = O> + MultiQuery<Key = O, Value = M>,
  );
  fn describe(&self, cx: &mut Context) -> Self::Compute {
    let (d, v, v2) = (**self).poll_changes_with_inv_dyn(cx);
    let v = OneManyRelationDualAccess {
      many_access_one: v,
      one_access_many: v2,
    };
    (d, v)
  }
  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.deref_mut().extra_request_dyn(request)
  }
}

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
