use crate::*;

pub trait DynReactiveOneToManyRelation<O: CKey, M: CKey>: Send + Sync {
  /// we could return a single trait object that cover both access and inverse access
  /// but for simplicity we just return two trait objects as these two trait both impl clone.
  fn poll_changes_with_inv_dyn(&self, cx: &mut Context)
    -> DynReactiveOneToManyRelationResult<M, O>;

  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation);
}

type DynReactiveOneToManyRelationResult<M, O> = Box<
  dyn Future<
      Output = (
        Box<dyn DynVirtualCollection<M, ValueChange<O>>>,
        Box<dyn DynVirtualCollection<M, O>>,
        Box<dyn DynVirtualMultiCollection<O, M>>,
      ),
    > + Unpin
    + Send,
>;

impl<O, M, T> DynReactiveOneToManyRelation<O, M> for T
where
  O: CKey,
  M: CKey,
  T: ReactiveOneToManyRelation<O, M>,
{
  fn poll_changes_with_inv_dyn(
    &self,
    cx: &mut Context,
  ) -> DynReactiveOneToManyRelationResult<M, O> {
    Box::new(
      self
        .poll_changes(cx)
        .map(|(d, v)| {
          (
            Box::new(d) as Box<dyn DynVirtualCollection<M, ValueChange<O>>>,
            Box::new(v.clone()) as Box<dyn DynVirtualCollection<M, O>>,
            Box::new(v) as Box<dyn DynVirtualMultiCollection<O, M>>,
          )
        })
        .boxed(),
    )
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
  type Task = impl Future<Output = (Self::Changes, Self::View)>;
  fn poll_changes(&self, cx: &mut Context) -> Self::Task {
    self.poll_changes_with_inv_dyn(cx).map(|(d, v, v2)| {
      let v = OneManyRelationDualAccess {
        many_access_one: v,
        one_access_many: v2,
      };
      (d, v)
    })
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
