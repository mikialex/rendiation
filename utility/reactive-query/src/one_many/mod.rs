mod bookkeeping;
pub use bookkeeping::*;

mod fanout;
mod map;
mod reduce;

mod dyn_impl;
pub use dyn_impl::*;
pub use fanout::*;
pub use map::*;
pub use reduce::*;

use crate::*;

/// this is just a helper trait to facilitate bound expression
pub trait ReactiveOneToManyRelationCompute:
  AsyncQueryCompute<
  Key = Self::Many,
  Value = Self::One,
  View: MultiQuery<Key = Self::One, Value = Self::Many>,
>
{
  type One: CKey;
  type Many: CKey;
}
impl<T> ReactiveOneToManyRelationCompute for T
where
  T: AsyncQueryCompute<View: MultiQuery<Key = T::Value, Value = T::Key>>,
  T::Value: CKey,
{
  type One = T::Value;
  type Many = T::Key;
}

/// this is just a helper trait to facilitate bound expression
pub trait ReactiveOneToManyRelation:
  ReactiveQuery<
  Key = Self::Many,
  Value = Self::One,
  Compute: ReactiveOneToManyRelationCompute<One = Self::One, Many = Self::Many>,
>
{
  type One: CKey;
  type Many: CKey;
}

impl<T> ReactiveOneToManyRelation for T
where
  T: ReactiveQuery<Compute: QueryCompute<View: MultiQuery<Key = T::Value, Value = T::Key>>>,
  T::Value: CKey,
{
  type One = T::Value;
  type Many = T::Key;
}

pub trait ReactiveOneToManyRelationExt: ReactiveOneToManyRelation + Sized {
  fn into_boxed_many_one_debug_large_symbol_workaround(
    self,
  ) -> impl ReactiveOneToManyRelation<One = Self::One, Many = Self::Many> {
    #[cfg(debug_assertions)]
    {
      Box::new(self) as BoxedDynReactiveOneToManyRelation<Self::One, Self::Many>
    }

    #[cfg(not(debug_assertions))]
    {
      self
    }
  }

  fn into_reactive_state_many_one(self) -> ReactiveManyOneRelationAsReactiveQuery<Self> {
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
  ) -> impl ReactiveOneToManyRelation<One = Self::One, Many = M2> {
    ReactiveKeyDualMapRelation {
      inner: self,
      f1: f,
      f2: f_v,
    }
  }
}
impl<T: ReactiveOneToManyRelation> ReactiveOneToManyRelationExt for T {}

pub trait ReactiveQueryOneToManyRelationExt: Sized + ReactiveQuery<Value: CKey> {
  fn into_one_to_many_by_hash(self) -> OneToManyRefHashBookKeeping<Self, Self::Key, Self::Value> {
    OneToManyRefHashBookKeeping {
      upstream: self,
      mapping: Default::default(),
    }
  }

  fn into_one_to_many_by_idx(self) -> OneToManyRefDenseBookKeeping<Self>
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
impl<T> ReactiveQueryOneToManyRelationExt for T where T: Sized + ReactiveQuery<Value: CKey> {}

pub trait ReactiveQueryRelationReduceExt: Sized + ReactiveQuery<Value = ()> {
  fn many_to_one_reduce_key<Relation>(
    self,
    relations: Relation,
  ) -> ManyToOneReduce<Self, Relation, Relation::Value>
  where
    Relation::Value: CKey,
    Relation: ReactiveQuery<Key = Self::Key>,
  {
    ManyToOneReduce {
      upstream: self,
      relations,
      ref_count: Default::default(),
    }
  }
}
impl<T> ReactiveQueryRelationReduceExt for T where T: Sized + ReactiveQuery<Value = ()> {}
