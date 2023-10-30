use std::{marker::PhantomData, sync::Arc};

use fast_hash_collection::{FastHashMap, FastHashSet};
use futures::task::{ArcWake, AtomicWaker};
use parking_lot::{RwLock, RwLockReadGuard};

use crate::*;

// post/async transform
// Vec<KVDelta<K, V>> ==(if V::Delta==V, single value)=> Vec<KVDelta<K, V>>
// Vec<KVDelta<K, V>> ==(drop any invalid in history + group by k)=> Vec<KVDelta<K, V>>

// sync reduce
// group single value
// group multi value

pub enum VirtualKVCollectionDelta<K, V> {
  /// here we not impose any delta on
  Delta(K, V),
  Remove(K),
}

impl<K, V> VirtualKVCollectionDelta<K, V> {
  pub fn map<R>(self, mapper: impl FnOnce(&K, V) -> R) -> VirtualKVCollectionDelta<K, R> {
    type Rt<K, R> = VirtualKVCollectionDelta<K, R>;
    match self {
      Self::Remove(k) => Rt::<K, R>::Remove(k),
      Self::Delta(k, d) => {
        let mapped = mapper(&k, d);
        Rt::<K, R>::Delta(k, mapped)
      }
    }
  }

  // should we just use struct??
  pub fn key(&self) -> &K {
    match self {
      Self::Remove(k) => k,
      Self::Delta(k, _) => k,
    }
  }
}

pub trait VirtualKVCollection<K, V> {
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_;

  fn iter_key_value(&self, skip_cache: bool) -> impl Iterator<Item = (K, V)> + '_ {
    let access = self.access(skip_cache);
    self.iter_key(skip_cache).map(move |k| {
      let v = access(&k).expect("iter_key_value provide key but not have valid value");
      (k, v)
    })
  }

  /// Access the current value. we use this scoped api style for fast batch accessing(avoid internal
  /// fragmented locking). the returned V is pass by ownership because we may create data on the
  /// fly.
  ///
  /// If the skip_cache is true, the implementation will not be incremental and will make sure the
  /// access is up to date. If the return is None, it means the value is not exist in the table.
  ///
  /// The implementation should guarantee it's ok to allow  multiple accessor instance exist in same
  /// time. (should only create read guard in underlayer)
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_;
}

/// An abstraction of reactive key-value like virtual container.
///
/// You can imagine this is a data table with the K as the primary key and V as the row of the
/// data(not contains K). In this table, besides getting data, you can also poll it's partial
/// changes.
///
/// Implementation notes:
///
/// This trait maybe could generalize to SignalLike trait:
/// ```rust
/// pub trait Signal<T: IncrementalBase>: Stream<Item = T::Delta> {
///   fn access(&self) -> T;
/// }
/// ```
/// However, this idea has not baked enough. For example, how do we express efficient partial
/// access for large T or container like T? Should we use some accessor associate trait or type as
/// the accessor key? Should we link this type to the T like how we did in Incremental trait?
pub trait ReactiveKVCollection<K, V>: VirtualKVCollection<K, V> + Stream + Unpin + 'static {}

/// The data maybe slate if we combine these two trait directly because the visitor maybe not
/// directly access the original source data, but access the cache. This access abstract the
/// internal cache mechanism. Note, even if the polling issued before access, you still can not
/// guaranteed to access the "current" data due to the multi-threaded source mutation. Because of
/// this limitation, user should make sure their downstream consuming logic is timeline insensitive.
///
/// In the future, maybe we could add new sub-trait to enforce the data access is consistent with
/// the polling logic in tradeoff of the potential memory overhead.
impl<T, K, V> ReactiveKVCollection<K, V> for T
where
  T: VirtualKVCollection<K, V> + Stream + Unpin + 'static,
  Self::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
}

/// dynamic version of the above trait
pub trait DynamicVirtualKVCollection<K, V> {
  fn iter_key_boxed(&self, skip_cache: bool) -> Box<dyn Iterator<Item = K> + '_>;
  fn access_boxed(&self, skip_cache: bool) -> Box<dyn Fn(&K) -> Option<V> + '_>;
}
impl<K, V, T> DynamicVirtualKVCollection<K, V> for T
where
  Self: ReactiveKVCollection<K, V>,
{
  fn iter_key_boxed(&self, skip_cache: bool) -> Box<dyn Iterator<Item = K> + '_> {
    Box::new(self.iter_key(skip_cache))
  }

  fn access_boxed(&self, skip_cache: bool) -> Box<dyn Fn(&K) -> Option<V> + '_> {
    Box::new(self.access(skip_cache))
  }
}
pub trait DynamicReactiveKVCollection<K, V>:
  DynamicVirtualKVCollection<K, V> + Stream<Item = Vec<VirtualKVCollectionDelta<K, V>>> + Unpin
{
}
impl<K, V> VirtualKVCollection<K, V> for &dyn DynamicReactiveKVCollection<K, V> {
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_ {
    self.access_boxed(skip_cache)
  }

  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    self.iter_key_boxed(skip_cache)
  }
}

pub trait ReactiveKVCollectionExt<K, V>: Sized + 'static + ReactiveKVCollection<K, V>
where
  Self::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
  /// map map<k, v> to map<k, v2>
  fn collective_map<V2, F: Fn(V) -> V2 + Copy>(self, f: F) -> ReactiveKVMap<Self, F, K, V2> {
    ReactiveKVMap {
      inner: self,
      map: f,
      phantom: PhantomData,
    }
  }

  /// filter map<k, v> by v
  fn collective_filter<F>(self, f: F) -> ReactiveKVFilter<Self, F, K> {
    ReactiveKVFilter {
      inner: self,
      checker: f,
      k: PhantomData,
    }
  }

  // /// filter map<k, v> by reactive set<k>
  // fn filter_by_keyset<S: ReactiveKVCollection<K, ()>>(
  //   self,
  //   set: S,
  // ) -> impl ReactiveKVCollection<K, V> {
  //   //
  // }

  fn collective_union<V2, Other>(self, other: Other) -> ReactiveKVUnion<Self, Other, K>
  where
    Other: ReactiveKVCollection<K, V2>,
    Other::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V2>>,
  {
    ReactiveKVUnion {
      a: self,
      b: other,
      k: PhantomData,
    }
  }

  fn collective_select<Other>(
    self,
    other: Other,
  ) -> ReactiveKVMap<ReactiveKVUnion<Self, Other, K>, Selector<V>, K, V>
  where
    K: Copy + std::hash::Hash + Eq + 'static,
    Other: ReactiveKVCollection<K, V>,
    Other::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
  {
    self.collective_union(other).collective_map(selector)
  }

  fn collective_zip<Other, V2>(
    self,
    other: Other,
  ) -> ReactiveKVMap<ReactiveKVUnion<Self, Other, K>, Zipper<V, V2>, K, (V, V2)>
  where
    K: Copy + std::hash::Hash + Eq + 'static,
    Other: ReactiveKVCollection<K, V2>,
    Other::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V2>>,
  {
    self.collective_union(other).collective_map(zipper)
  }

  fn materialize_unordered(self) -> UnorderedMaterializedReactiveKVCollection<Self, K, V> {
    UnorderedMaterializedReactiveKVCollection {
      inner: self,
      cache: Default::default(),
    }
  }
}
impl<T, K, V> ReactiveKVCollectionExt<K, V> for T
where
  T: Sized + 'static + ReactiveKVCollection<K, V>,
  Self::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
}

fn selector<T>((a, b): (Option<T>, Option<T>)) -> T {
  match (a, b) {
    (Some(_), Some(_)) => unreachable!("key set should not overlap"),
    (Some(a), None) => a,
    (None, Some(b)) => b,
    (None, None) => unreachable!("value not selected"),
  }
}

fn zipper<T, U>((a, b): (Option<T>, Option<U>)) -> (T, U) {
  match (a, b) {
    (Some(a), Some(b)) => (a, b),
    _ => unreachable!("value not zipped"),
  }
}

type Selector<T> = impl Fn((Option<T>, Option<T>)) -> T;
type Zipper<T, U> = impl Fn((Option<T>, Option<U>)) -> (T, U);

// pub struct ReactiveKVMapFork<Map> {
//   inner: Arc<RwLock<Map>>,
//   wakers: Arc<WakerBroadcast>,
//   id: u64,
// }

// struct WakerBroadcast {
//   wakers: RwLock<FastHashMap<u64, AtomicWaker>>,
// }
// impl ArcWake for WakerBroadcast {
//   fn wake_by_ref(arc_self: &Arc<Self>) {
//     let wakers = arc_self.wakers.read();
//     for w in wakers.values() {
//       w.wake()
//     }
//   }
// }

// impl<Map> Drop for ReactiveKVMapFork<Map> {
//   fn drop(&mut self) {
//     self.wakers.wakers.write().remove(&self.id);
//   }
// }
// impl<Map> Clone for ReactiveKVMapFork<Map> {
//   fn clone(&self) -> Self {
//     self.wakers.clone().wake();
//     Self {
//       inner: self.inner.clone(),
//       wakers: self.wakers.clone(),
//       id: alloc_global_res_id(),
//     }
//   }
// }

// impl<Map: Stream + Unpin> Stream for ReactiveKVMapFork<Map> {
//   type Item = Map::Item;

//   fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//     // these writes should not deadlock, because we not prefer the concurrency between the table
//     // updates. if we do allow it in the future, just change it to try write or yield pending.

//     {
//       let mut wakers = self.wakers.wakers.write();
//       let waker = wakers.entry(self.id).or_insert_with(Default::default);
//       waker.register(cx.waker());
//     }

//     let waker = futures::task::waker_ref(&self.wakers);
//     let mut cx = std::task::Context::from_waker(&waker);

//     let mut inner = self.inner.write();
//     inner.poll_next_unpin(&mut cx)
//   }
// }

// impl<K, V, Map> VirtualKVCollection<K, V> for ReactiveKVMapFork<Map>
// where
//   Map: VirtualKVCollection<K, V> + 'static,
// {
//   fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
//     struct ReactiveKVMapForkRead<'a, Map, I> {
//       _inner: RwLockReadGuard<'a, Map>,
//       inner_iter: I,
//     }

//     impl<'a, Map, I: Iterator> Iterator for ReactiveKVMapForkRead<'a, Map, I> {
//       type Item = I::Item;

//       fn next(&mut self) -> Option<Self::Item> {
//         self.inner_iter.next()
//       }
//     }

//     /// util to get accessor type
//     type IterOf<'a, M: VirtualKVCollection<K, V> + 'a, K, V> = impl Iterator<Item = K> + 'a;
//     fn get_iter<'a, K, V, M>(map: &M, skip_cache: bool) -> IterOf<M, K, V>
//     where
//       M: VirtualKVCollection<K, V> + 'a,
//     {
//       map.iter_key(skip_cache)
//     }

//     let inner = self.inner.read();
//     let inner_iter = get_iter(inner.deref(), skip_cache);
//     // safety: read guard is hold by iter, acc's real reference is form the Map
//     let inner_iter: IterOf<'static, Map, K, V> = unsafe { std::mem::transmute(inner_iter) };
//     ReactiveKVMapForkRead {
//       _inner: inner,
//       inner_iter,
//     }
//   }

//   fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_ {
//     let inner = self.inner.read();

//     /// util to get accessor type
//     type AccessorOf<'a, M: VirtualKVCollection<K, V> + 'a, K, V> = impl Fn(&K) -> Option<V> + 'a;
//     fn get_accessor<'a, K, V, M>(map: &M, skip_cache: bool) -> AccessorOf<M, K, V>
//     where
//       M: VirtualKVCollection<K, V> + 'a,
//     {
//       map.access(skip_cache)
//     }

//     let acc: AccessorOf<Map, K, V> = get_accessor(inner.deref(), skip_cache);
//     // safety: read guard is hold by closure, acc's real reference is form the Map
//     let acc: AccessorOf<'static, Map, K, V> = unsafe { std::mem::transmute(acc) };
//     move |key| {
//       let _holder = &inner;
//       let acc = &acc;
//       acc(key)
//     }
//   }
// }

#[pin_project::pin_project]
pub struct UnorderedMaterializedReactiveKVCollection<Map, K, V> {
  #[pin]
  inner: Map,
  cache: FastHashMap<K, V>,
}

impl<Map, K, V> Stream for UnorderedMaterializedReactiveKVCollection<Map, K, V>
where
  Map: Stream,
  K: std::hash::Hash + Eq,
  V: IncrementalBase<Delta = V>,
  Map::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>> + Clone,
{
  type Item = Map::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let r = this.inner.poll_next(cx);
    if let Poll::Ready(Some(changes)) = &r {
      for change in changes.clone().into_iter() {
        match change {
          VirtualKVCollectionDelta::Delta(k, v) => {
            this.cache.insert(k, v);
          }
          VirtualKVCollectionDelta::Remove(k) => {
            // todo, shrink
            this.cache.remove(&k);
          }
        }
      }
    }
    r
  }
}

impl<K, V, Map> VirtualKVCollection<K, V> for UnorderedMaterializedReactiveKVCollection<Map, K, V>
where
  Map: VirtualKVCollection<K, V>,
  K: std::hash::Hash + Eq + Clone,
  V: Clone,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    if skip_cache {
      Box::new(self.inner.iter_key(skip_cache)) as Box<dyn Iterator<Item = K> + '_>
    } else {
      Box::new(self.cache.keys().cloned()) as Box<dyn Iterator<Item = K> + '_>
    }
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_ {
    let inner = self.inner.access(skip_cache);
    move |key| {
      if skip_cache {
        inner(key)
      } else {
        self.cache.get(key).cloned()
      }
    }
  }
}

#[pin_project::pin_project]
pub struct ReactiveKVMap<T, F, K, V> {
  #[pin]
  inner: T,
  map: F,
  phantom: PhantomData<(K, V)>,
}

impl<T, F, K, V, V2> Stream for ReactiveKVMap<T, F, K, V>
where
  F: Fn(V) -> V2 + Copy + 'static,
  T: Stream,
  T::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
  type Item = impl IntoIterator<Item = VirtualKVCollectionDelta<K, V2>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let mapper = *this.map;
    this.inner.poll_next(cx).map(move |r| {
      r.map(move |deltas| {
        deltas
          .into_iter()
          .map(move |delta| delta.map(|_, v| mapper(v)))
      })
    })
  }
}

impl<T, F, K, V, V2> VirtualKVCollection<K, V2> for ReactiveKVMap<T, F, K, V>
where
  F: Fn(V) -> V2 + Copy,
  T: VirtualKVCollection<K, V>,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    self.inner.iter_key(skip_cache)
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V2> + '_ {
    let inner_getter = self.inner.access(skip_cache);
    move |key| inner_getter(key).map(|v| (self.map)(v))
  }
}

#[pin_project::pin_project]
pub struct ReactiveKVFilter<T, F, K> {
  #[pin]
  inner: T,
  checker: F,
  k: PhantomData<K>,
}

impl<T, F, K, V> Stream for ReactiveKVFilter<T, F, K>
where
  F: Fn(&V) -> bool + Copy + 'static,
  T: Stream,
  T::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
  type Item = impl IntoIterator<Item = VirtualKVCollectionDelta<K, V>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let checker = *this.checker;
    this.inner.poll_next(cx).map(move |r| {
      r.map(move |deltas| {
        deltas.into_iter().filter(move |delta| match delta {
          VirtualKVCollectionDelta::Delta(_, v) => checker(v),
          // the Remove variant maybe called many times for given k
          VirtualKVCollectionDelta::Remove(_) => true,
        })
      })
    })
  }
}

impl<T, F, K, V> VirtualKVCollection<K, V> for ReactiveKVFilter<T, F, K>
where
  F: Fn(&V) -> bool + Copy,
  T: VirtualKVCollection<K, V>,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    let inner_getter = self.inner.access(skip_cache);
    self.inner.iter_key(skip_cache).filter(move |k| {
      let v = inner_getter(k).unwrap();
      (self.checker)(&v)
    })
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_ {
    let inner_getter = self.inner.access(skip_cache);
    move |key| inner_getter(key).and_then(|v| (self.checker)(&v).then_some(v))
  }
}

#[pin_project::pin_project]
pub struct ReactiveKSetFilter<T, KS, K> {
  #[pin]
  inner: T,
  #[pin]
  keys: KS,
  k: PhantomData<K>,
}

impl<T, KS, K, V> Stream for ReactiveKSetFilter<T, KS, K>
where
  T: Stream,
  T::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
  KS: ReactiveKVCollection<K, ()>,
{
  type Item = impl IntoIterator<Item = VirtualKVCollectionDelta<K, V>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let keys_change = this.keys.poll_next(cx);
    let inner_change = this.inner.poll_next(cx);
    // todo!()
    Poll::Ready(Some(Vec::new()))
  }
}

impl<T, KS, K, V> VirtualKVCollection<K, V> for ReactiveKSetFilter<T, KS, K>
where
  KS: ReactiveKVCollection<K, ()>,
  T: VirtualKVCollection<K, V>,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    self.keys.iter_key(skip_cache)
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<V> + '_ {
    let keys_acc = self.keys.access(skip_cache);
    let inner_getter = self.inner.access(skip_cache);
    move |key| keys_acc(key).and_then(|_| inner_getter(key))
  }
}

#[pin_project::pin_project]
pub struct ReactiveKVUnion<T1, T2, K> {
  #[pin]
  a: T1,
  #[pin]
  b: T2,
  k: PhantomData<K>,
}

impl<T1, T2, K, V1, V2> VirtualKVCollection<K, (Option<V1>, Option<V2>)>
  for ReactiveKVUnion<T1, T2, K>
where
  K: Copy + std::hash::Hash + Eq,
  T1: VirtualKVCollection<K, V1>,
  T2: VirtualKVCollection<K, V2>,
{
  /// we require the T1 T2 has the same key range
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_ {
    let mut keys = FastHashSet::<K>::default();
    self.a.iter_key(skip_cache).for_each(|k| {
      keys.insert(k);
    });
    self.b.iter_key(skip_cache).for_each(|k| {
      keys.insert(k);
    });
    keys.into_iter()
  }
  fn access(&self, skip_cache: bool) -> impl Fn(&K) -> Option<(Option<V1>, Option<V2>)> + '_ {
    let getter_a = self.a.access(skip_cache);
    let getter_b = self.b.access(skip_cache);

    move |key| Some((getter_a(key), getter_b(key)))
  }
}

impl<T1, T2, K, V1, V2> Stream for ReactiveKVUnion<T1, T2, K>
where
  K: Clone,
  T1: Stream + Unpin,
  T1::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V1>>,
  T2: Stream + Unpin,
  T2::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V2>>,
  T1: VirtualKVCollection<K, V1>,
  T2: VirtualKVCollection<K, V2>,
{
  type Item = impl IntoIterator<Item = VirtualKVCollectionDelta<K, (Option<V1>, Option<V2>)>>;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let t1 = self.a.poll_next_unpin(cx);
    let t2 = self.b.poll_next_unpin(cx);

    let a_access = self.a.access(false);
    let b_access = self.b.access(false);

    match (t1, t2) {
      (Poll::Ready(Some(v1)), Poll::Ready(Some(v2))) => {
        let intersections: FastHashMap<K, (Option<V1>, Option<V2>)> = FastHashMap::default();
        v1.into_iter().for_each(|d| {
          match d {
            VirtualKVCollectionDelta::Delta(K, V) => {
              //
            }
            VirtualKVCollectionDelta::Remove(K) => {
              //
            }
          }
        });

        Poll::Ready(Some(Vec::new()))
      }
      (Poll::Ready(Some(v1)), Poll::Pending) => Poll::Ready(Some(
        v1.into_iter()
          .map(|v1| v1.map(|k, v| (Some(v), b_access(k))))
          .collect::<Vec<_>>(),
      )),
      (Poll::Pending, Poll::Ready(Some(v2))) => Poll::Ready(Some(
        v2.into_iter()
          .map(|v2| v2.map(|k, v| (a_access(k), Some(v))))
          .collect::<Vec<_>>(),
      )),
      (Poll::Pending, Poll::Pending) => Poll::Pending,
      _ => Poll::Ready(None),
    }
  }
}
