mod fork;
pub use fork::*;

mod union;
pub use union::*;

mod delta_merge;
pub use delta_merge::*;

mod cache;
pub use cache::*;

mod map;
pub use map::*;

mod filter;
pub use filter::*;

mod utils;
pub use utils::*;

use crate::*;

#[pin_project::pin_project]
struct ReactiveCollectionAsStream<T, K, V> {
  #[pin]
  inner: T,
  phantom: PhantomData<(K, V)>,
}

impl<K, V, T> Stream for ReactiveCollectionAsStream<T, K, V>
where
  T: ReactiveCollection<K, V> + Unpin,
  K: CKey,
  V: CValue,
{
  type Item = Arc<FastHashMap<K, ValueChange<V>>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let r = this.inner.poll_changes(cx);
    loop {
      match r {
        CPoll::Ready(r) => return r.map(|delta| Some(delta.materialize())),
        CPoll::Blocked => continue,
      }
    }
  }
}

pub trait ReactiveCollectionExtForAcc<K, V>: ReactiveCollection<K, V>
where
  V: CValue,
  K: CKey,
{
  fn make_accessor(&self) -> impl Fn(&K) -> Option<V> + '_ {
    let view = self.spin_get_current();
    move |k| view.access(k)
  }
}
impl<T, K, V> ReactiveCollectionExtForAcc<K, V> for T
where
  T: ReactiveCollection<K, V> + ?Sized,
  V: CValue,
  K: CKey,
{
}

pub trait ReactiveCollectionExt<K, V>: ReactiveCollection<K, V>
where
  V: CValue,
  K: CKey,
  Self: Sized + 'static,
{
  fn into_boxed(self) -> Box<dyn ReactiveCollection<K, V>>
  where
    Self: Sized + 'static,
  {
    Box::new(self)
  }

  fn into_change_stream(self) -> impl Stream<Item = Arc<FastHashMap<K, ValueChange<V>>>>
  where
    Self: Unpin,
  {
    ReactiveCollectionAsStream {
      inner: self,
      phantom: PhantomData,
    }
  }

  #[inline(always)]
  fn workaround_box(self) -> impl ReactiveCollection<K, V> {
    let r = self;
    // this is a workaround that the compiler maybe generate huge outputs(like pdb file)  which lead
    // to link error in debug build, as well as using huge memory
    // see https://doc.rust-lang.org/reference/conditional-compilation.html#debug_assertions
    #[cfg(debug_assertions)]
    let r = r.into_boxed();

    r
  }

  /// map map<k, v> to map<k, v2>
  fn collective_map<V2, F>(self, f: F) -> impl ReactiveCollection<K, V2>
  where
    F: Fn(V) -> V2 + Copy + Send + Sync + 'static,
    V2: CValue,
  {
    ReactiveKVMap {
      inner: self,
      map: f,
      phantom: PhantomData,
    }
    .workaround_box()
  }

  /// map map<k, v> to map<k, v2>
  fn collective_execute_map_by<V2, F, FF>(self, f: F) -> impl ReactiveCollection<K, V2>
  where
    F: Fn() -> FF + Send + Sync + 'static,
    FF: Fn(&K, V) -> V2 + Send + Sync + 'static,
    V2: CValue,
  {
    ReactiveKVExecuteMap {
      inner: self,
      map_creator: f,
      cache: Default::default(),
      phantom: PhantomData,
    }
    .workaround_box()
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
    .workaround_box()
  }

  fn collective_union<V2, Other, F, O>(self, other: Other, f: F) -> impl ReactiveCollection<K, O>
  where
    Other: ReactiveCollection<K, V2>,
    V2: CValue,
    O: CValue,
    F: Fn((Option<V>, Option<V2>)) -> Option<O> + Send + Sync + Copy + 'static,
  {
    ReactiveKVUnion {
      a: BufferedCollection::new(self),
      b: BufferedCollection::new(other),
      phantom: PhantomData,
      f,
    }
    .workaround_box()
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
      (None, None) => unreachable!("value not selected"),
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
      _ => unreachable!("value not zipped"),
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
    BufferedCollection::new(ReactiveKVMapForkImpl::new(self))
  }

  /// project map<O, V> -> map<M, V> when we have O - M one to many
  fn one_to_many_fanout<MK, Relation>(self, relations: Relation) -> impl ReactiveCollection<MK, V>
  where
    MK: CKey,
    Relation: ReactiveOneToManyRelationship<K, MK> + 'static,
  {
    OneToManyFanout {
      upstream: BufferedCollection::new(self),
      relations: BufferedCollection::new(relations),
      phantom: PhantomData,
    }
    .workaround_box()
  }

  fn materialize_unordered(self) -> impl ReactiveCollection<K, V>
  where
    K: CKey,
  {
    UnorderedMaterializedReactiveCollection {
      inner: self,
      cache: Default::default(),
    }
    .workaround_box()
  }
  fn materialize_linear(self) -> impl ReactiveCollection<K, V>
  where
    K: LinearIdentification + CKey,
  {
    LinearMaterializedReactiveCollection {
      inner: self,
      cache: Default::default(),
    }
    .workaround_box()
  }

  fn diff_change(self) -> impl ReactiveCollection<K, V>
  where
    K: std::fmt::Debug + CKey,
    V: std::fmt::Debug + CValue + PartialEq,
  {
    ReactiveCollectionDiff {
      inner: self,
      phantom: Default::default(),
    }
    .workaround_box()
  }

  fn debug(self, label: &'static str) -> impl ReactiveCollection<K, V>
  where
    K: std::fmt::Debug + CKey,
    V: std::fmt::Debug + CValue + PartialEq,
  {
    ReactiveCollectionDebug {
      inner: self,
      state: Default::default(),
      label,
    }
    .workaround_box()
  }
}
impl<T, K, V> ReactiveCollectionExt<K, V> for T
where
  T: ReactiveCollection<K, V> + Sized + 'static,
  V: CValue,
  K: CKey,
{
}
