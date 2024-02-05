use crate::*;

pub struct RelationOneSideFilter<R, OF> {
  relation: R,
  filter_set: OF,
}

impl<M: CKey, O: CKey, R, OF> ReactiveCollection<M, O> for RelationOneSideFilter<R, OF>
where
  R: ReactiveOneToManyRelationship<O, M>,
  OF: ReactiveCollection<O, ()>,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<M, O> {
    todo!()
  }

  fn access(&self) -> PollCollectionCurrent<M, O> {
    // extra check if m get o result is exist in o set
    todo!()
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    todo!()
  }
}

impl<M: CKey, O: CKey, R, OF> ReactiveOneToManyRelationship<O, M> for RelationOneSideFilter<R, OF>
where
  R: ReactiveOneToManyRelationship<O, M>,
  OF: ReactiveCollection<O, ()>,
{
  fn multi_access(&self) -> Box<dyn VirtualMultiCollection<O, M> + '_> {
    // check if o is in the o set or skip
    todo!()
  }
}
