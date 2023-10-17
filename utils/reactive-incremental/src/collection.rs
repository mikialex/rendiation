use std::marker::PhantomData;

use crate::*;

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
    //    match self{
    //     Self::Insert(k, v) =>  Self::Insert(k, v),
    //     Self::Remove(k) => todo!(),
    //     Self::Delta(k, d) => todo!(),
    // }
    todo!()
  }
}

/// An abstraction of reactive key-value like virtual container.
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
  // fn map<V2>(self, f: impl Fn(V) -> V2) -> impl ReactiveKVCollection<K, V2> {
  //   //
  // }
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
      getter(&|key| {
        inner_getter(key).map(|v| {
          if let MaybeDelta::All(r) = (self.map)(MaybeDelta::All(v)) {
            r
          } else {
            unreachable!()
          }
        })
      })
    })
  }
}
