use crate::*;

type MutationData<T> = (Vec<u32>, FastHashMap<u32, T>);

/// this should be a cheaper version of collective_channel
/// todo, improve code sharing with collective channel or use more advance solution
pub fn changes_channel<T>() -> (ChangesMutationSender<T>, ChangesMutationReceiver<T>) {
  let inner: Arc<(RwLock<MutationData<T>>, AtomicWaker)> = Default::default();
  let sender = ChangesMutationSender {
    inner: inner.clone(),
  };
  let receiver = ChangesMutationReceiver { inner };

  (sender, receiver)
}

pub struct ChangesMutationSender<T> {
  inner: Arc<(RwLock<MutationData<T>>, AtomicWaker)>,
}

impl<T> Clone for ChangesMutationSender<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

use parking_lot::lock_api::RawRwLock;
impl<T: CValue> ChangesMutationSender<T> {
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
    let mutations = &mut *self.inner.0.data_ptr();
    if !(mutations.0.is_empty() && mutations.1.is_empty()) {
      self.inner.1.wake();
    }
    self.inner.0.raw().unlock_exclusive()
  }
  /// # Safety
  ///
  /// this should be called when locked
  pub unsafe fn send(&self, idx: u32, change: Option<T>) {
    let mutations = &mut *self.inner.0.data_ptr();

    if let Some(new) = change {
      mutations.1.insert(idx, new);
    } else {
      mutations.0.push(idx);
      mutations.1.remove(&idx);
    }
  }
  /// # Safety
  ///
  /// this should be called when locked
  pub unsafe fn reserve_space(&self, size: usize) {
    let mutations = &mut *self.inner.0.data_ptr();
    mutations.0.reserve(size);
    mutations.1.reserve(size);
  }

  pub fn is_closed(&self) -> bool {
    // self inner is shared between sender and receiver, if not shared anymore it must be
    // receiver not exist anymore, so the channel is closed.
    Arc::strong_count(&self.inner) == 1
  }
}

/// this is not likely to be triggered because component type is not get removed in any time
impl<T> Drop for ChangesMutationSender<T> {
  fn drop(&mut self) {
    self.inner.1.wake()
  }
}

pub struct ChangesMutationReceiver<T> {
  inner: Arc<(RwLock<MutationData<T>>, AtomicWaker)>,
}

impl<T: CValue> ChangesMutationReceiver<T> {
  pub fn poll_impl(&self, cx: &mut Context) -> Poll<Option<MutationData<T>>> {
    self.inner.1.register(cx.waker());
    let mut changes = self.inner.0.write();
    let changes: &mut MutationData<T> = &mut changes;

    let changes = std::mem::take(changes);
    if !(changes.0.is_empty() && changes.1.is_empty()) {
      Poll::Ready(Some(changes))
      // check if the sender has been dropped
    } else if Arc::strong_count(&self.inner) == 1 {
      Poll::Ready(None)
    } else {
      Poll::Pending
    }
  }
  pub fn has_change(&self) -> bool {
    let changes = self.inner.0.read();
    !changes.0.is_empty()
  }
}

impl<T: CValue> Stream for ChangesMutationReceiver<T> {
  type Item = MutationData<T>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.poll_impl(cx)
  }
}

pub(crate) fn add_changes_listen<T: CValue>(
  query: impl QueryProvider<RawEntityHandle, T>,
  source: &EventSource<ChangePtr>,
) -> ChangesMutationReceiver<T> {
  let (sender, receiver) = changes_channel::<T>();
  // expand initial value while first listen.
  unsafe {
    sender.lock();
    let query = query.access();
    let iter = query.iter_key_value();

    let count_hint = iter.size_hint().0;
    sender.reserve_space(count_hint);

    for (idx, v) in iter {
      sender.send(idx.alloc_index(), Some(v));
    }
    sender.unlock();
  }

  source.on(move |change| unsafe {
    match change {
      ScopedMessage::Start => {
        sender.lock();
        false
      }
      ScopedMessage::End => {
        sender.unlock();
        sender.is_closed()
      }
      ScopedMessage::ReserveSpace(size) => {
        sender.reserve_space(*size);
        false
      }
      ScopedMessage::Message(write) => {
        let change = write
          .change
          .new_value()
          .map(|v| (*(v.0 as *const T)).clone());
        sender.send(write.idx.alloc_index(), change);
        false
      }
    }
  });
  receiver
}
