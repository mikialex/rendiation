mod bookkeeping;
pub use bookkeeping::*;

mod map;
mod projection;

mod dyn_impl;
pub use dyn_impl::*;
pub use map::*;
pub use projection::*;

use crate::*;

pub trait ReactiveOneToManyRelation<O: CKey, M: CKey>:
  ReactiveCollection<M, O, View: VirtualMultiCollection<O, M>>
{
}

impl<O: CKey, M: CKey, T> ReactiveOneToManyRelation<O, M> for T where
  T: ReactiveCollection<M, O, View: VirtualMultiCollection<O, M>>
{
}

pub trait ReactiveOneToManyRelationExt<O: CKey, M: CKey>: ReactiveOneToManyRelation<O, M> {
  fn into_reactive_state_many_one(self) -> impl ReactiveQuery<Output = Box<dyn std::any::Any>>
  where
    Self: Sized,
  {
    ReactiveManyOneRelationAsReactiveQuery {
      inner: self,
      phantom: PhantomData,
    }
  }

  fn map_value<M2: CKey>(self, f: impl Fn(&M) -> M2) -> impl ReactiveOneToManyRelation<O, M2>
  where
    Self: Sized,
  {
    todo!()
  }
  fn dual_map_key<O2: CKey>(
    self,
    f: impl Fn(&O) -> O2,
    f_v: impl Fn(&O2) -> O,
  ) -> impl ReactiveOneToManyRelation<O2, M>
  where
    Self: Sized,
  {
    todo!()
  }
}
impl<O: CKey, M: CKey, T: ReactiveOneToManyRelation<O, M>> ReactiveOneToManyRelationExt<O, M>
  for T
{
}

pub trait ReactiveCollectionRelationExt<K: CKey, V: CKey>:
  Sized + ReactiveCollection<K, V>
{
  fn into_one_to_many_by_hash(self) -> impl ReactiveOneToManyRelation<V, K>
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

  fn into_one_to_many_by_idx(self) -> impl ReactiveOneToManyRelation<V, K>
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
