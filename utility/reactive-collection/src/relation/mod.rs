mod bookkeeping;
pub use bookkeeping::*;

mod projection;
use std::ops::DerefMut;

pub use projection::*;

use crate::*;

pub trait ReactiveOneToManyRelationship<O: CKey, M: CKey>: ReactiveCollection<M, O> {
  fn multi_access(&self) -> Box<dyn VirtualMultiCollection<O, M> + '_>;
}

pub trait ReactiveOneToManyRelationshipExt<O: CKey, M: CKey>:
  ReactiveOneToManyRelationship<O, M>
{
  fn make_multi_accessor(&self) -> impl Fn(&O, &mut dyn FnMut(M)) + Send + Sync + '_ {
    let view = self.multi_access();
    move |k, visitor| view.access_multi(k, visitor)
  }
}
impl<O: CKey, M: CKey, T: ReactiveOneToManyRelationship<O, M>>
  ReactiveOneToManyRelationshipExt<O, M> for T
{
}

impl<O, M> ReactiveCollection<M, O> for Box<dyn ReactiveOneToManyRelationship<O, M>>
where
  O: CKey,
  M: CKey,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<M, O> {
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
  fn multi_access(&self) -> Box<dyn VirtualMultiCollection<O, M> + '_> {
    self.deref().multi_access()
  }
}

pub trait ReactiveCollectionRelationExt<K: CKey, V: CKey>:
  Sized + ReactiveCollection<K, V>
{
  fn into_one_to_many_by_hash(self) -> impl ReactiveOneToManyRelationship<V, K>
  where
    K: CKey,
    V: CKey,
  {
    OneToManyRefHashBookKeeping {
      upstream: self,
      mapping: Default::default(),
    }
  }

  fn into_one_to_many_by_hash_expose_type(self) -> OneToManyRefHashBookKeeping<V, K, Self>
  where
    K: CKey,
    V: CKey,
  {
    OneToManyRefHashBookKeeping {
      upstream: self,
      mapping: Default::default(),
    }
  }

  fn into_one_to_many_by_idx(self) -> impl ReactiveOneToManyRelationship<V, K>
  where
    K: CKey + LinearIdentification,
    V: CKey + LinearIdentification,
  {
    OneToManyRefDenseBookKeeping {
      upstream: self,
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
      upstream: self,
      mapping: Default::default(),
      phantom: PhantomData,
    }
  }
}
impl<T, K, V> ReactiveCollectionRelationExt<K, V> for T
where
  T: Sized + ReactiveCollection<K, V>,
  K: CKey,
  V: CKey,
{
}

pub trait ReactiveCollectionRelationReduceExt<K: CKey>: Sized + ReactiveCollection<K, ()> {
  fn many_to_one_reduce_key<SK, Relation>(
    self,
    relations: Relation,
  ) -> impl ReactiveCollection<SK, ()>
  where
    SK: CKey,
    K: CKey,
    Relation: ReactiveCollection<K, SK>,
  {
    ManyToOneReduce {
      upstream: self,
      relations,
      phantom: PhantomData,
      ref_count: Default::default(),
    }
  }
}
impl<T, K: CKey> ReactiveCollectionRelationReduceExt<K> for T where
  T: Sized + ReactiveCollection<K, ()>
{
}
