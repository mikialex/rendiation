use crate::*;

pub trait DynReactiveOneToManyRelation<O: CKey, M: CKey>: Send + Sync {
  /// we could return a single trait object that cover both access and inverse access
  /// but for simplicity we just return two trait objects as these two trait both impl clone.
  fn poll_changes_with_inv_dyn(
    &self,
    cx: &mut Context,
  ) -> (
    Box<dyn DynVirtualCollection<M, ValueChange<O>>>,
    Box<dyn DynVirtualCollection<M, O>>,
    Box<dyn DynVirtualMultiCollection<O, M>>,
  );

  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation);
}

impl<O, M, T> DynReactiveOneToManyRelation<O, M> for T
where
  O: CKey,
  M: CKey,
  T: ReactiveOneToManyRelation<O, M>,
{
  fn poll_changes_with_inv_dyn(
    &self,
    cx: &mut Context,
  ) -> (
    Box<dyn DynVirtualCollection<M, ValueChange<O>>>,
    Box<dyn DynVirtualCollection<M, O>>,
    Box<dyn DynVirtualMultiCollection<O, M>>,
  ) {
    let (d, v) = self.poll_changes(cx);
    (Box::new(d), Box::new(v.clone()), Box::new(v))
  }

  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation) {
    self.extra_request(request)
  }
}

impl<O, M> ReactiveCollection<M, O> for Box<dyn DynReactiveOneToManyRelation<O, M>>
where
  O: CKey,
  M: CKey,
{
  type Changes = impl VirtualCollection<M, ValueChange<O>>;
  type View = impl VirtualCollection<M, O> + VirtualMultiCollection<O, M>;
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, v, v2) = (**self).poll_changes_with_inv_dyn(cx);
    let v = OneManyRelationDualAccess {
      many_access_one: v,
      one_access_many: v2,
    };
    (d, v)
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.deref_mut().extra_request_dyn(request)
  }
}

#[derive(Clone)]
pub struct OneManyRelationDualAccess<T, IT> {
  pub many_access_one: T,
  pub one_access_many: IT,
}

impl<O, M, T, IT> VirtualCollection<M, O> for OneManyRelationDualAccess<T, IT>
where
  O: CKey,
  M: CKey,
  T: VirtualCollection<M, O>,
  IT: VirtualMultiCollection<O, M>,
{
  fn iter_key_value(&self) -> impl Iterator<Item = (M, O)> + '_ {
    self.many_access_one.iter_key_value()
  }

  fn access(&self, key: &M) -> Option<O> {
    self.many_access_one.access(key)
  }
}

impl<O, M, T, IT> VirtualMultiCollection<O, M> for OneManyRelationDualAccess<T, IT>
where
  O: CKey,
  M: CKey,
  T: VirtualCollection<M, O>,
  IT: VirtualMultiCollection<O, M>,
{
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = O> + '_ {
    self.one_access_many.iter_key_in_multi_collection()
  }

  fn access_multi(&self, key: &O) -> Option<impl Iterator<Item = M> + '_> {
    self.one_access_many.access_multi(key)
  }
}
