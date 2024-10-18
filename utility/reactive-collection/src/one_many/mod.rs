mod bookkeeping;
pub use bookkeeping::*;

mod map;
mod projection;

mod dyn_impl;
pub use dyn_impl::*;
pub use map::*;
pub use projection::*;

use crate::*;

pub trait ReactiveOneToManyRelation:
  ReactiveCollection<
  Key = Self::Many,
  Value = Self::One,
  View: VirtualMultiCollection<Key = Self::One, Value = Self::Many>,
>
{
  type One: CKey;
  type Many: CKey;
}

impl<T> ReactiveOneToManyRelation for T
where
  T: ReactiveCollection<View: VirtualMultiCollection<Key = T::Value, Value = T::Key>>,
  T::Value: CKey,
{
  type One = T::Value;
  type Many = T::Key;
}

pub trait ReactiveOneToManyRelationExt: ReactiveOneToManyRelation {
  fn into_reactive_state_many_one(self) -> impl ReactiveQuery<Output = Box<dyn std::any::Any>>
  where
    Self: Sized,
  {
    ReactiveManyOneRelationAsReactiveQuery { inner: self }
  }

  fn collective_map_key_one_many<O2, F, F2>(
    self,
    f: F,
    f2: F2,
  ) -> impl ReactiveOneToManyRelation<One = O2, Many = Self::Many>
  where
    F: Fn(Self::One) -> O2 + Copy + Send + Sync + 'static,
    F2: Fn(O2) -> Self::One + Copy + Send + Sync + 'static,
    O2: CKey,
    Self: Sized,
  {
    ReactiveKVMapRelation {
      inner: self,
      map: move |_: &_, v| f(v),
      f1: f,
      f2,
    }
  }

  fn collective_dual_map_one_many<M2: CKey>(
    self,
    f: impl Fn(Self::Many) -> M2 + Copy + 'static + Send + Sync,
    f_v: impl Fn(M2) -> Self::Many + Copy + 'static + Send + Sync,
  ) -> impl ReactiveOneToManyRelation<One = Self::One, Many = M2>
  where
    Self: Sized,
  {
    ReactiveKeyDualMapRelation {
      inner: self,
      f1: f,
      f2: f_v,
    }
  }
}
impl<T: ReactiveOneToManyRelation> ReactiveOneToManyRelationExt for T {}

pub trait ReactiveCollectionRelationExt: Sized + ReactiveCollection<Value: CKey> {
  fn into_one_to_many_by_hash(
    self,
  ) -> impl ReactiveOneToManyRelation<One = Self::Value, Many = Self::Key> {
    OneToManyRefHashBookKeeping {
      upstream: self,
      mapping: Default::default(),
    }
  }

  fn into_one_to_many_by_hash_expose_type(self) -> OneToManyRefHashBookKeeping<Self> {
    OneToManyRefHashBookKeeping {
      upstream: self,
      mapping: Default::default(),
    }
  }

  fn into_one_to_many_by_idx(
    self,
  ) -> impl ReactiveOneToManyRelation<One = Self::Value, Many = Self::Key>
  where
    Self::Key: LinearIdentification,
    Self::Value: LinearIdentification,
  {
    OneToManyRefDenseBookKeeping {
      upstream: self,
      mapping: Default::default(),
    }
  }

  fn into_one_to_many_by_idx_expose_type(self) -> OneToManyRefDenseBookKeeping<Self>
  where
    Self::Key: LinearIdentification,
    Self::Value: LinearIdentification,
  {
    OneToManyRefDenseBookKeeping {
      upstream: self,
      mapping: Default::default(),
    }
  }
}
impl<T> ReactiveCollectionRelationExt for T where T: Sized + ReactiveCollection<Value: CKey> {}

pub trait ReactiveCollectionRelationReduceExt: Sized + ReactiveCollection<Value = ()> {
  fn many_to_one_reduce_key<SK, Relation>(
    self,
    relations: Relation,
  ) -> impl ReactiveCollection<Key = SK, Value = ()>
  where
    SK: CKey,
    Relation: ReactiveCollection<Key = Self::Key, Value = SK>,
  {
    ManyToOneReduce {
      upstream: self,
      relations,
      ref_count: Default::default(),
    }
  }
}
impl<T> ReactiveCollectionRelationReduceExt for T where T: Sized + ReactiveCollection<Value = ()> {}