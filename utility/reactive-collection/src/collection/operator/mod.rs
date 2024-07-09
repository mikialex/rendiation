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

mod utils;
pub use utils::*;

use crate::*;

pub trait ReactiveCollectionExt<K, V>: ReactiveCollection<K, V>
where
  V: CValue,
  K: CKey,
  Self: Sized + 'static,
{
  fn into_boxed(self) -> Box<dyn DynReactiveCollection<K, V>>
  where
    Self: Sized + 'static,
  {
    Box::new(self)
  }

  fn into_reactive_state(self) -> impl ReactiveQuery<Output = Box<dyn std::any::Any>> {
    ReactiveCollectionAsReactiveQuery {
      inner: self,
      phantom: PhantomData,
    }
  }

  fn into_change_stream(
    self,
  ) -> impl futures::Stream<Item = Box<dyn Future<Output = FastHashMap<K, ValueChange<V>>>>>
  where
    Self: Unpin,
  {
    ReactiveCollectionAsStream {
      inner: self,
      phantom: PhantomData,
    }
  }

  fn key_as_value(self) -> impl ReactiveCollection<K, K> {
    self.collective_kv_map(|k, _| k.clone())
  }

  fn collective_key_dual_map<K2: CKey>(
    self,
    f: impl Fn(K) -> K2 + Copy + 'static + Send + Sync,
    f2: impl Fn(K2) -> K + Copy + 'static + Send + Sync,
  ) -> impl ReactiveCollection<K2, V> {
    ReactiveKeyDualMap {
      f1: f,
      f2,
      inner: self,
      phantom: PhantomData,
    }
  }

  /// map map<k, v> to map<k, v2>
  fn collective_kv_map<V2, F>(self, f: F) -> impl ReactiveCollection<K, V2>
  where
    F: Fn(&K, V) -> V2 + Copy + Send + Sync + 'static,
    V2: CValue,
  {
    ReactiveKVMap {
      inner: self,
      map: f,
      phantom: PhantomData,
    }
  }

  /// map map<k, v> to map<k, v2>
  fn collective_map<V2, F>(self, f: F) -> impl ReactiveCollection<K, V2>
  where
    F: Fn(V) -> V2 + Copy + Send + Sync + 'static,
    V2: CValue,
  {
    ReactiveKVMap {
      inner: self,
      map: move |_: &_, v| f(v),
      phantom: PhantomData,
    }
  }

  /// map map<k, v> to map<k, v2>
  fn collective_execute_map_by<V2, F, FF>(self, f: F) -> impl ReactiveCollection<K, V2>
  where
    F: Fn() -> FF + Send + Sync + Clone + 'static,
    FF: Fn(&K, V) -> V2 + Send + Sync + 'static,
    V2: CValue,
  {
    ReactiveKVExecuteMap {
      inner: self,
      map_creator: f,
      cache: Default::default(),
      phantom: PhantomData,
    }
  }

  /// filter map<k, v> by v
  fn collective_filter<F>(self, f: F) -> impl ReactiveCollection<K, V>
  where
    V: Copy,
    F: Fn(V) -> bool + Copy + Send + Sync + 'static,
  {
    ReactiveKVFilter {
      inner: self,
      checker: move |v| if f(v) { Some(v) } else { None },
      k: PhantomData,
    }
  }

  /// filter map<k, v> by v
  fn collective_filter_map<V2, F>(self, f: F) -> impl ReactiveCollection<K, V2>
  where
    F: Fn(V) -> Option<V2> + Copy + Send + Sync + 'static,
    V2: CValue,
  {
    ReactiveKVFilter {
      inner: self,
      checker: f,
      k: PhantomData,
    }
  }

  fn collective_cross_join<K2, V2>(
    self,
    other: impl ReactiveCollection<K2, V2>,
  ) -> impl ReactiveCollection<(K, K2), (V, V2)>
  where
    K2: CKey,
    V2: CValue,
  {
    ReactiveCrossJoin {
      a: self,
      b: other,
      phantom: PhantomData,
    }
  }

  fn collective_union<V2, Other, F, O>(self, other: Other, f: F) -> impl ReactiveCollection<K, O>
  where
    Other: ReactiveCollection<K, V2>,
    V2: CValue,
    O: CValue,
    F: Fn((Option<V>, Option<V2>)) -> Option<O> + Send + Sync + Copy + 'static,
  {
    ReactiveKVUnion {
      a: self,
      b: other,
      phantom: PhantomData,
      f,
    }
  }

  /// K should not overlap
  fn collective_select<Other>(self, other: Other) -> impl ReactiveCollection<K, V>
  where
    Other: ReactiveCollection<K, V>,
  {
    self.collective_union(other, |(a, b)| match (a, b) {
      (Some(_), Some(_)) => unreachable!("key set should not overlap"),
      (Some(a), None) => a.into(),
      (None, Some(b)) => b.into(),
      (None, None) => None,
    })
  }

  /// K should fully overlap
  fn collective_zip<Other, V2>(self, other: Other) -> impl ReactiveCollection<K, (V, V2)>
  where
    Other: ReactiveCollection<K, V2>,
    V2: CValue,
  {
    self.collective_union(other, |(a, b)| match (a, b) {
      (Some(a), Some(b)) => Some((a, b)),
      (None, None) => None,
      (None, Some(_)) => unreachable!("zip missing left side"),
      (Some(_), None) => unreachable!("zip missing right side"),
    })
  }

  /// only return overlapped part
  fn collective_intersect<Other, V2>(self, other: Other) -> impl ReactiveCollection<K, (V, V2)>
  where
    Other: ReactiveCollection<K, V2>,
    V2: CValue,
  {
    self.collective_union(other, |(a, b)| match (a, b) {
      (Some(a), Some(b)) => Some((a, b)),
      _ => None,
    })
  }

  /// filter map<k, v> by reactive set<k>
  /// have to use box here to avoid complex type(could be improved)
  fn filter_by_keyset<S>(self, set: S) -> impl ReactiveCollection<K, V>
  where
    S: ReactiveCollection<K, ()>,
  {
    self.collective_union(set, |(a, b)| match (a, b) {
      (Some(a), Some(_)) => Some(a),
      _ => None,
    })
  }

  fn into_forker(self) -> ReactiveKVMapFork<Self, K, V> {
    ReactiveKVMapFork::new(self, false)
  }

  fn into_static_forker(self) -> ReactiveKVMapFork<Self, K, V> {
    ReactiveKVMapFork::new(self, true)
  }

  /// project map<O, V> -> map<M, V> when we have O - M one to many
  fn one_to_many_fanout<MK, Relation>(self, relations: Relation) -> impl ReactiveCollection<MK, V>
  where
    MK: CKey,
    Relation: ReactiveOneToManyRelation<K, MK> + 'static,
  {
    OneToManyFanout {
      upstream: self,
      relations,
      phantom: PhantomData,
    }
  }

  fn materialize_unordered(self) -> UnorderedMaterializedReactiveCollection<Self, K, V>
  where
    K: CKey,
  {
    UnorderedMaterializedReactiveCollection {
      inner: self,
      cache: Default::default(),
    }
  }
  fn materialize_linear(self) -> LinearMaterializedReactiveCollection<Self, V>
  where
    K: LinearIdentification + CKey,
  {
    LinearMaterializedReactiveCollection {
      inner: self,
      cache: Default::default(),
    }
  }

  fn diff_change(self) -> impl ReactiveCollection<K, V>
  where
    K: CKey,
    V: CValue,
  {
    ReactiveCollectionDiff {
      inner: self,
      phantom: Default::default(),
    }
  }

  fn debug(self, label: &'static str) -> impl ReactiveCollection<K, V>
  where
    K: CKey,
    V: CValue,
  {
    ReactiveCollectionDebug {
      inner: self,
      state: Default::default(),
      label,
    }
  }
}
impl<T, K, V> ReactiveCollectionExt<K, V> for T
where
  T: ReactiveCollection<K, V> + Sized + 'static,
  V: CValue,
  K: CKey,
{
}
