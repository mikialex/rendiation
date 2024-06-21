use crate::*;

type MutationData<K, T> = FastHashMap<K, ValueChange<T>>;

pub fn collective_channel<K, T>() -> (
  CollectiveMutationSender<K, T>,
  CollectiveMutationReceiver<K, T>,
) {
  let inner: Arc<(RwLock<MutationData<K, T>>, AtomicWaker)> = Default::default();
  let sender = CollectiveMutationSender {
    inner: inner.clone(),
  };
  let receiver = CollectiveMutationReceiver { inner };

  (sender, receiver)
}

pub struct CollectiveMutationSender<K, T> {
  inner: Arc<(RwLock<MutationData<K, T>>, AtomicWaker)>,
}

impl<K, T> Clone for CollectiveMutationSender<K, T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<K: CKey, T: CValue> CollectiveMutationSender<K, T> {
  /// # Safety
  ///
  /// this should be called before send
  pub unsafe fn lock(&self) {
    self.inner.0.raw().lock_exclusive()
  }
  /// # Safety
  ///
  /// this should be called after send
  pub unsafe fn unlock(&self) {
    self.inner.1.wake();
    self.inner.0.raw().unlock_exclusive()
  }
  /// # Safety
  ///
  /// this should be called when locked
  pub unsafe fn send(&self, idx: K, change: ValueChange<T>) {
    let mutations = &mut *self.inner.0.data_ptr();
    if let Some(old_change) = mutations.get_mut(&idx) {
      if !old_change.merge(&change) {
        mutations.remove(&idx);
      }
    } else {
      mutations.insert(idx, change);
    }
  }
  pub fn is_closed(&self) -> bool {
    // self inner is shared between sender and receiver, if not shared anymore it must be
    // receiver not exist anymore, so the channel is closed.
    Arc::strong_count(&self.inner) == 1
  }
}

/// this is not likely to be triggered because component type is not get removed in any time
impl<K, T> Drop for CollectiveMutationSender<K, T> {
  fn drop(&mut self) {
    self.inner.1.wake()
  }
}

pub struct CollectiveMutationReceiver<K, T> {
  inner: Arc<(RwLock<MutationData<K, T>>, AtomicWaker)>,
}

impl<K: CKey, T: CValue> CollectiveMutationReceiver<K, T> {
  pub fn poll_impl(
    &self,
    cx: &mut Context,
  ) -> Poll<Option<Box<dyn DynVirtualCollection<K, ValueChange<T>>>>> {
    self.inner.1.register(cx.waker());
    let mut changes = self.inner.0.write();
    let changes: &mut MutationData<K, T> = &mut changes;

    let changes = std::mem::take(changes);
    if !changes.is_empty() {
      Poll::Ready(Some(Box::new(changes)))
      // check if the sender has been dropped
    } else if Arc::strong_count(&self.inner) == 1 {
      Poll::Ready(None)
    } else {
      Poll::Pending
    }
  }
}

impl<K: CKey, T: CValue> Stream for CollectiveMutationReceiver<K, T> {
  type Item = Box<dyn DynVirtualCollection<K, ValueChange<T>>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.poll_impl(cx)
  }
}

// this trait could be lift into upper stream
pub trait VirtualCollectionProvider<K, V>: Send + Sync {
  fn access(&self) -> CollectionView<K, V>;
}

impl<K: CKey, V: CValue, T: VirtualCollection<K, V> + 'static> VirtualCollectionProvider<K, V>
  for Arc<RwLock<T>>
{
  fn access(&self) -> CollectionView<K, V> {
    Box::new(self.make_read_holder())
  }
}

pub struct ReactiveCollectionFromCollectiveMutation<K, T> {
  pub full: Box<dyn VirtualCollectionProvider<K, T>>,
  pub mutation: RwLock<CollectiveMutationReceiver<K, T>>,
}
impl<K: CKey, T: CValue> ReactiveCollection<K, T>
  for ReactiveCollectionFromCollectiveMutation<K, T>
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, T> {
    match self.mutation.write().poll_next_unpin(cx) {
      Poll::Ready(Some(r)) => Poll::Ready(r),
      _ => Poll::Pending,
    }
  }

  fn access(&self) -> PollCollectionCurrent<K, T> {
    self.full.access()
  }

  fn extra_request(&mut self, _request: &mut ExtraCollectionOperation) {
    // component storage should not shrink here
  }
}
