mod fork;
pub use fork::*;

mod union;
pub use union::*;

mod cache;
pub use cache::*;

mod map;
pub use map::*;

mod filter;
pub use filter::*;

mod join;
pub use join::*;

mod reverse;
pub use reverse::*;

mod utils;
pub use utils::*;

use crate::*;

pub trait ReactiveQueryExt: ReactiveQuery
where
  Self: Sized + 'static,
{
  fn into_boxed(self) -> BoxedDynReactiveQuery<Self::Key, Self::Value> {
    Box::new(self)
  }

  fn into_reactive_state(self) -> impl ReactiveGeneralQuery<Output = Box<dyn std::any::Any>> {
    ReactiveQueryAsReactiveGeneralQuery { inner: self }
  }

  fn into_change_stream(
    self,
  ) -> impl futures::Stream<Item = Arc<FastHashMap<Self::Key, ValueChange<Self::Value>>>>
  where
    Self: Unpin,
  {
    ReactiveQueryAsStream { inner: self }
  }

  fn key_as_value(self) -> impl ReactiveQuery<Key = Self::Key, Value = Self::Key> {
    self.collective_kv_map(|k, _| k.clone())
  }

  fn hash_reverse_assume_one_one(self) -> impl ReactiveQuery<Key = Self::Value, Value = Self::Key>
  where
    Self::Value: CKey,
  {
    OneToOneRefHashBookKeeping {
      upstream: self,
      mapping: Default::default(),
    }
  }

  fn collective_key_dual_map<K2: CKey>(
    self,
    f: impl Fn(Self::Key) -> K2 + Copy + 'static + Send + Sync,
    f2: impl Fn(K2) -> Self::Key + Copy + 'static + Send + Sync,
  ) -> impl ReactiveQuery<Key = K2, Value = Self::Value> {
    ReactiveKeyDualMap {
      f1: f,
      f2,
      inner: self,
    }
  }

  /// map map<k, v> to map<k, v2>
  fn collective_kv_map<V2, F>(self, f: F) -> impl ReactiveQuery<Key = Self::Key, Value = V2>
  where
    F: Fn(&Self::Key, Self::Value) -> V2 + Copy + Send + Sync + 'static,
    V2: CValue,
  {
    ReactiveKVMap {
      inner: self,
      map: f,
    }
  }

  /// map map<k, v> to map<k, v2>
  fn collective_map<V2, F>(self, f: F) -> impl ReactiveQuery<Key = Self::Key, Value = V2>
  where
    F: Fn(Self::Value) -> V2 + Copy + Send + Sync + 'static,
    V2: CValue,
  {
    ReactiveKVMap {
      inner: self,
      map: move |_: &_, v| f(v),
    }
  }

  /// map map<k, v> to map<k, v2>
  fn collective_execute_map_by<V2, F, FF>(
    self,
    f: F,
  ) -> impl ReactiveQuery<Key = Self::Key, Value = V2>
  where
    F: Fn() -> FF + Send + Sync + 'static,
    FF: FnMut(&Self::Key, Self::Value) -> V2 + Send + Sync + 'static,
    V2: CValue,
  {
    ReactiveKVExecuteMap {
      inner: self,
      map_creator: f,
      cache: Default::default(),
    }
  }

  /// filter map<k, v> by v
  fn collective_filter<F>(self, f: F) -> impl ReactiveQuery<Key = Self::Key, Value = Self::Value>
  where
    F: Fn(Self::Value) -> bool + Clone + Send + Sync + 'static,
  {
    ReactiveKVFilter {
      inner: self,
      checker: move |v: Self::Value| if f(v.clone()) { Some(v) } else { None }, // todo remove clone
    }
  }

  /// filter map<k, v> by v
  fn collective_filter_map<V2, F>(self, f: F) -> impl ReactiveQuery<Key = Self::Key, Value = V2>
  where
    F: Fn(Self::Value) -> Option<V2> + Copy + Send + Sync + 'static,
    V2: CValue,
  {
    ReactiveKVFilter {
      inner: self,
      checker: f,
    }
  }

  fn collective_cross_join<O>(
    self,
    other: O,
  ) -> impl ReactiveQuery<Key = (Self::Key, O::Key), Value = (Self::Value, O::Value)>
  where
    O: ReactiveQuery,
  {
    ReactiveCrossJoin { a: self, b: other }
  }

  fn collective_union<Other, F, O>(
    self,
    other: Other,
    f: F,
  ) -> impl ReactiveQuery<Key = Self::Key, Value = O>
  where
    Other: ReactiveQuery<Key = Self::Key>,
    O: CValue,
    F: Fn((Option<Self::Value>, Option<Other::Value>)) -> Option<O> + Send + Sync + Copy + 'static,
  {
    ReactiveKVUnion {
      a: self,
      b: other,
      f,
    }
    .into_boxed() // todo, remove this in release build
  }

  /// K should not overlap
  fn collective_select<Other>(
    self,
    other: Other,
  ) -> impl ReactiveQuery<Key = Self::Key, Value = Self::Value>
  where
    Other: ReactiveQuery<Key = Self::Key, Value = Self::Value>,
  {
    self.collective_union(other, |(a, b)| match (a, b) {
      (Some(_), Some(_)) => unreachable!("key set should not overlap"),
      (Some(a), None) => a.into(),
      (None, Some(b)) => b.into(),
      (None, None) => None,
    })
  }

  /// K should fully overlap
  fn collective_zip<Other>(
    self,
    other: Other,
  ) -> impl ReactiveQuery<Key = Self::Key, Value = (Self::Value, Other::Value)>
  where
    Other: ReactiveQuery<Key = Self::Key>,
  {
    self.collective_union(other, |(a, b)| match (a, b) {
      (Some(a), Some(b)) => Some((a, b)),
      (None, None) => None,
      (None, Some(_)) => unreachable!("zip missing left side"),
      (Some(_), None) => unreachable!("zip missing right side"),
    })
  }

  /// only return overlapped part
  fn collective_intersect<Other>(
    self,
    other: Other,
  ) -> impl ReactiveQuery<Key = Self::Key, Value = (Self::Value, Other::Value)>
  where
    Other: ReactiveQuery<Key = Self::Key>,
  {
    self.collective_union(other, |(a, b)| match (a, b) {
      (Some(a), Some(b)) => Some((a, b)),
      _ => None,
    })
  }

  /// filter map<k, v> by reactive set<k>
  /// have to use box here to avoid complex type(could be improved)
  fn filter_by_keyset<S>(self, set: S) -> impl ReactiveQuery<Key = Self::Key, Value = Self::Value>
  where
    S: ReactiveQuery<Key = Self::Key, Value = ()>,
  {
    self.collective_union(set, |(a, b)| match (a, b) {
      (Some(a), Some(_)) => Some(a),
      _ => None,
    })
  }

  fn into_forker(self) -> ReactiveKVMapFork<Self> {
    ReactiveKVMapFork::new(self, false)
  }

  fn into_static_forker(self) -> ReactiveKVMapFork<Self> {
    ReactiveKVMapFork::new(self, true)
  }

  /// project map<O, V> -> map<M, V> when we have O - M one to many
  fn one_to_many_fanout<Relation>(
    self,
    relations: Relation,
  ) -> impl ReactiveQuery<Key = Relation::Many, Value = Self::Value>
  where
    Relation: ReactiveOneToManyRelation<One = Self::Key> + 'static,
  {
    OneToManyFanout {
      upstream: self,
      relations,
    }
  }

  fn materialize_unordered(self) -> UnorderedMaterializedReactiveQuery<Self> {
    UnorderedMaterializedReactiveQuery {
      inner: self,
      cache: Default::default(),
    }
  }
  fn materialize_linear(self) -> LinearMaterializedReactiveQuery<Self>
  where
    Self::Key: LinearIdentification,
  {
    LinearMaterializedReactiveQuery {
      inner: self,
      cache: Default::default(),
    }
  }

  fn diff_change(self) -> impl ReactiveQuery<Key = Self::Key, Value = Self::Value> {
    ReactiveQueryDiff { inner: self }
  }

  fn debug(
    self,
    label: &'static str,
    log_change: bool,
  ) -> impl ReactiveQuery<Key = Self::Key, Value = Self::Value> {
    ReactiveQueryDebug {
      inner: self,
      state: Default::default(),
      label,
      log_change,
    }
  }
}
impl<T> ReactiveQueryExt for T where T: ReactiveQuery + Sized + 'static {}
