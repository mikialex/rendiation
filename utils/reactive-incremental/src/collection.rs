use std::marker::PhantomData;

use crate::*;

// post/async transform
// Vec<KVDelta<K, V>> ==(if V::Delta==V, single value)=> Vec<KVDelta<K, V>>
// Vec<KVDelta<K, V>> ==(drop any invalid in history + group by k)=> Vec<KVDelta<K, V>>

// sync reduce
// group single value
// group multi value

pub enum VirtualKVCollectionDelta<K, V: IncrementalBase> {
  Insert(K, V),
  Remove(K),
  Delta(K, V::Delta),
}

impl<K, V: IncrementalBase> VirtualKVCollectionDelta<K, V> {
  pub fn map<R: IncrementalBase>(
    self,
    mapper: impl FnOnce(MaybeDelta<V>) -> MaybeDelta<R>,
  ) -> VirtualKVCollectionDelta<K, R> {
    type Rt<K, R> = VirtualKVCollectionDelta<K, R>;
    match self {
      Self::Insert(k, v) => Rt::Insert(k, mapper(MaybeDelta::All(v)).expect_all()),
      Self::Remove(k) => Rt::<K, R>::Remove(k),
      Self::Delta(k, d) => Rt::<K, R>::Delta(k, mapper(MaybeDelta::Delta(d)).expect_delta()),
    }
  }
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
pub trait ReactiveKVCollection<K, V: IncrementalBase>:
  Stream<Item = Vec<VirtualKVCollectionDelta<K, V>>> + Unpin
{
  /// Access the current value. we use this scoped api style for fast batch accessing(avoid internal
  /// fragmented locking). the returned V is pass by ownership because we may create data on the
  /// fly.
  ///
  /// The data maybe slate because the visitor maybe not directly access the original source data,
  /// but access the cache. This access abstract the internal cache mechanism. Note, even if the
  /// polling issued before access, you still can not guaranteed to access the "current" data due to
  /// the multi-threaded source mutation. Because of this limitation, user should make sure their
  /// downstream consuming logic is timeline insensitive.
  ///
  /// In the future, maybe we could add new sub-trait to enforce the data access is consistent with
  /// the polling logic in tradeoff of the potential memory overhead.
  fn access(&self, getter: impl FnOnce(&dyn Fn(K) -> Option<V>));
}

pub trait ReactiveKVCollectionExt<K, V: IncrementalBase>:
  Sized + 'static + ReactiveKVCollection<K, V>
{
  fn map<V2>(
    self,
    f: impl Fn(MaybeDelta<V>) -> MaybeDelta<V2> + Copy,
  ) -> impl ReactiveKVCollection<K, V2>
  where
    V: IncrementalBase,
    V2: IncrementalBase,
  {
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
impl<T, K, V: IncrementalBase> ReactiveKVCollectionExt<K, V> for T where
  T: Sized + 'static + ReactiveKVCollection<K, V>
{
}

#[pin_project::pin_project]
struct ReactiveKVMap<T, F, K, V> {
  #[pin]
  inner: T,
  map: F,
  k: PhantomData<K>,
  pre_v: PhantomData<V>,
}

impl<T, F, K, V, V2> Stream for ReactiveKVMap<T, F, K, V>
where
  F: Fn(MaybeDelta<V>) -> MaybeDelta<V2> + Copy,
  T: ReactiveKVCollection<K, V>,
  V: IncrementalBase,
  V2: IncrementalBase,
{
  type Item = Vec<VirtualKVCollectionDelta<K, V2>>;

  // in current implementation, each map operator will do a allocation and data movement, could we
  // avoid this cost?
  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.inner.poll_next(cx).map(|r| {
      r.map(|deltas| {
        deltas
          .into_iter()
          .map(|delta| delta.map(|v| (this.map)(v)))
          .collect()
      })
    })
  }
}

impl<T, F, K, V, V2> ReactiveKVCollection<K, V2> for ReactiveKVMap<T, F, K, V>
where
  F: Fn(MaybeDelta<V>) -> MaybeDelta<V2> + Copy,
  T: ReactiveKVCollection<K, V>,
  V: IncrementalBase,
  V2: IncrementalBase,
  V2: Send + Sync,
{
  fn access(&self, getter: impl FnOnce(&dyn Fn(K) -> Option<V2>)) {
    self.inner.access(move |inner_getter| {
      getter(&|key| inner_getter(key).map(|v| (self.map)(MaybeDelta::All(v)).expect_all()))
    })
  }
}
