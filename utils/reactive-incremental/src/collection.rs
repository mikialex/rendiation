use std::marker::PhantomData;

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
  pub fn map<R>(self, mapper: impl FnOnce(V) -> R) -> VirtualKVCollectionDelta<K, R> {
    type Rt<K, R> = VirtualKVCollectionDelta<K, R>;
    match self {
      Self::Remove(k) => Rt::<K, R>::Remove(k),
      Self::Delta(k, d) => Rt::<K, R>::Delta(k, mapper(d)),
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
  /// Access the current value. we use this scoped api style for fast batch accessing(avoid internal
  /// fragmented locking). the returned V is pass by ownership because we may create data on the
  /// fly.
  fn access(&self, getter: impl FnOnce(&dyn Fn(K) -> Option<V>));
}

/// An abstraction of reactive key-value like virtual container.
///
/// This trait maybe could generalize to SignalLike trait:
/// ```rust
/// pub trait Signal<T: IncrementalBase>: Stream<Item = T::Delta> {
///   fn access(&self) -> T;
/// }
/// ```
/// However, this idea has is not baked enough. For example, how do we express efficient partial
/// access for large T or container like T? Should we use some accessor associate trait or type as
/// the accessor key? Should we link this type to the T like how we did in Incremental trait?
pub trait ReactiveKVCollection<K, V>: VirtualKVCollection<K, V> + Stream + Unpin
where
  Self::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
}

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
  T: VirtualKVCollection<K, V> + Stream + Unpin,
  Self::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
}

pub trait ReactiveKVCollectionExt<K, V>: Sized + 'static + ReactiveKVCollection<K, V>
where
  Self::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
  fn map<V2, F: Fn(V) -> V2 + Copy>(self, f: F) -> ReactiveKVMap<Self, F, K, V2> {
    ReactiveKVMap {
      inner: self,
      map: f,
      k: PhantomData,
      pre_v: PhantomData,
    }
  }
  // fn zip<V2>(
  //   self,
  //   other: impl ReactiveKVCollection<K, V2>,
  // ) -> impl ReactiveKVCollection<K, (V, V2)> {
  //   //
  // }
}
impl<T, K, V> ReactiveKVCollectionExt<K, V> for T
where
  T: Sized + 'static + ReactiveKVCollection<K, V>,
  Self::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
}

#[pin_project::pin_project]
pub struct ReactiveKVMap<T, F, K, V> {
  #[pin]
  inner: T,
  map: F,
  k: PhantomData<K>,
  pre_v: PhantomData<V>,
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
    this
      .inner
      .poll_next(cx)
      .map(move |r| r.map(move |deltas| deltas.into_iter().map(move |delta| delta.map(mapper))))
  }
}

impl<T, F, K, V, V2> VirtualKVCollection<K, V2> for ReactiveKVMap<T, F, K, V>
where
  F: Fn(V) -> V2 + Copy,
  T: VirtualKVCollection<K, V>,
{
  fn access(&self, getter: impl FnOnce(&dyn Fn(K) -> Option<V2>)) {
    self
      .inner
      .access(move |inner_getter| getter(&|key| inner_getter(key).map(|v| (self.map)(v))))
  }
}
