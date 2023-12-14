mod bookkeeping;
pub use bookkeeping::*;

mod projection;
use std::ops::DerefMut;

pub use projection::*;

use crate::*;

pub trait ReactiveOneToManyRelationship<O: Send, M: Send>: ReactiveCollection<M, O> {
  fn multi_access(&self) -> CPoll<Box<dyn VirtualMultiCollection<O, M> + '_>>;
}

impl<O, M> ReactiveCollection<M, O> for Box<dyn ReactiveOneToManyRelationship<O, M>>
where
  O: CKey,
  M: CKey,
{
  fn poll_changes(&self, cx: &mut Context<'_>) -> PollCollectionChanges<M, O> {
    self.deref().poll_changes(cx)
  }
  fn access(&self) -> PollCollectionCurrent<M, O> {
    self.deref().access()
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.deref_mut().extra_request(request)
  }
}

impl<O, M> ReactiveOneToManyRelationship<O, M> for Box<dyn ReactiveOneToManyRelationship<O, M>>
where
  O: CKey,
  M: CKey,
{
  fn multi_access(&self) -> CPoll<Box<dyn VirtualMultiCollection<O, M> + '_>> {
    self.deref().multi_access()
  }
}

pub trait ReactiveCollectionRelationExt<K: Send, V: Send>:
  Sized + 'static + ReactiveCollection<K, V>
{
  fn into_one_to_many_by_hash(self) -> impl ReactiveOneToManyRelationship<V, K>
  where
    K: CKey,
    V: CKey,
  {
    OneToManyRefHashBookKeeping {
      upstream: BufferedCollection::new(self),
      mapping: Default::default(),
    }
  }

  fn into_one_to_many_by_hash_expose_type(self) -> OneToManyRefHashBookKeeping<V, K, Self>
  where
    K: CKey,
    V: CKey,
  {
    OneToManyRefHashBookKeeping {
      upstream: BufferedCollection::new(self),
      mapping: Default::default(),
    }
  }

  fn into_one_to_many_by_idx(self) -> impl ReactiveOneToManyRelationship<V, K>
  where
    K: CKey + LinearIdentification,
    V: CKey + LinearIdentification,
  {
    OneToManyRefDenseBookKeeping {
      upstream: BufferedCollection::new(self),
      mapping: Default::default(),
      phantom: PhantomData,
    }
  }

  fn into_one_to_many_by_idx_expose_type(self) -> OneToManyRefDenseBookKeeping<V, K, Self>
  where
    K: CKey + LinearIdentification,
    V: CKey + LinearIdentification,
  {
    OneToManyRefDenseBookKeeping {
      upstream: BufferedCollection::new(self),
      mapping: Default::default(),
      phantom: PhantomData,
    }
  }
}
impl<T, K, V> ReactiveCollectionRelationExt<K, V> for T
where
  T: Sized + 'static + ReactiveCollection<K, V>,
  K: Send,
  V: Send,
{
}

pub trait ReactiveCollectionRelationReduceExt<K: Send>:
  Sized + 'static + ReactiveCollection<K, ()>
{
  fn many_to_one_reduce_key<SK, Relation>(
    self,
    relations: Relation,
  ) -> impl ReactiveCollection<SK, ()>
  where
    SK: Clone + Eq + Hash + Send + Sync + 'static,
    K: Clone + Eq + Hash + Sync + 'static,
    Relation: ReactiveCollection<K, SK> + 'static,
  {
    ManyToOneReduce {
      upstream: BufferedCollection::new(self),
      relations: BufferedCollection::new(relations),
      phantom: PhantomData,
      ref_count: Default::default(),
    }
  }
}
impl<T, K: Send> ReactiveCollectionRelationReduceExt<K> for T where
  T: Sized + 'static + ReactiveCollection<K, ()>
{
}
