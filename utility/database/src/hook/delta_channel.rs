use crate::*;

type MutationData<T> = FastDeltaChangeCollector<T>;

/// this should be a cheaper version of collective_channel
/// todo, improve code sharing with collective channel or use more advance solution
pub fn delta_channel<T: CValue>(
  bitmap_init: usize,
  change_init: usize,
) -> (ChangesMutationSender<T>, ChangesMutationReceiver<T>) {
  let inner = Arc::new((
    RwLock::new(MutationData::new(bitmap_init, change_init)),
    AtomicWaker::new(),
  ));
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
    if mutations.has_change() {
      self.inner.1.wake();
    }
    self.inner.0.raw().unlock_exclusive()
  }
  /// # Safety
  ///
  /// this should be called when locked
  pub unsafe fn send(&self, idx: RawEntityHandle, change: ValueChange<T>) {
    let mutations = &mut *self.inner.0.data_ptr();

    mutations.update_delta(idx, change);
  }
  /// # Safety
  ///
  /// this should be called when locked
  pub unsafe fn reserve_space(&self, size: usize) {
    let mutations = &mut *self.inner.0.data_ptr();
    mutations.reserve(size);
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
  pub fn poll_impl(&self, cx: &mut Context) -> Poll<Option<FastIterQuery<T>>> {
    self.inner.1.register(cx.waker());
    let mut changes = self.inner.0.write();

    let changes = changes.take_and_compute_query();
    if !changes.is_empty() {
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
    changes.has_change()
  }
}

impl<T: CValue> Stream for ChangesMutationReceiver<T> {
  type Item = FastIterQuery<T>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.poll_impl(cx)
  }
}

pub(crate) fn add_delta_listen<T: CValue>(
  bitmap_init: usize,
  query: impl QueryProvider<RawEntityHandle, T>,
  source: &EventSource<ChangePtr>,
) -> ChangesMutationReceiver<T> {
  let (sender, receiver) = delta_channel::<T>(bitmap_init, 0);
  // expand initial value while first listen.
  unsafe {
    sender.lock();
    let query = query.access();
    let iter = query.iter_key_value();

    let count_hint = iter.size_hint().0;
    sender.reserve_space(count_hint);

    for (idx, v) in iter {
      sender.send(idx, ValueChange::Delta(v, None));
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
        let change = write.change.map(|v| (*(v.0 as *const T)).clone());
        sender.send(write.idx, change);
        false
      }
    }
  });
  receiver
}

/// the optimization assumes: between the updates, one component is only changed once
/// in this case, this collector can avoid delta merge and data value move
///
/// note, using this collector will buffer the max history that can increase peak memory usage.
#[derive(Clone)]
pub struct FastDeltaChangeCollector<T> {
  has_any_change: Bitmap,
  has_duplicate_changes: Bitmap,
  unique_changed_key_count: usize,
  changes: Vec<(RawEntityHandle, ValueChange<T>)>,
}

impl<T: CValue> FastDeltaChangeCollector<T> {
  pub fn new(bitmap_init: usize, change_init: usize) -> Self {
    Self {
      has_any_change: Bitmap::with_size(bitmap_init),
      has_duplicate_changes: Bitmap::with_size(bitmap_init),
      unique_changed_key_count: 0,
      changes: Vec::with_capacity(change_init),
    }
  }

  pub fn reserve(&mut self, additional: usize) {
    self.changes.reserve(additional);
  }

  pub fn has_change(&self) -> bool {
    // todo
    self.unique_changed_key_count > 0
  }

  pub fn update_delta(&mut self, idx: RawEntityHandle, change: ValueChange<T>) {
    let index = idx.index() as usize;
    self.has_any_change.check_grow(index);
    self.has_duplicate_changes.check_grow(index);
    self.changes.push((idx, change));
    unsafe {
      let changed = self.has_any_change.get(index);
      if changed {
        self.has_duplicate_changes.set(index, true);
      } else {
        self.unique_changed_key_count += 1;
        self.has_any_change.set(index, true);
      }
    }
  }

  pub fn take_and_compute_query(&mut self) -> FastIterQuery<T> {
    if self.unique_changed_key_count == 0 {
      assert!(self.changes.is_empty());
      return FastIterQuery::empty();
    }

    let mut mapping = FastHashMap::with_capacity_and_hasher(
      self.unique_changed_key_count,
      FastHasherBuilder::default(),
    );
    let mut changes = std::mem::take(&mut self.changes);
    unsafe {
      let mut i = 0;
      let mut holes_indices = FastHashSet::default();

      loop {
        if i < changes.len() {
          let (key, change) = changes.get_unchecked(i);
          if self.has_duplicate_changes.get(i) {
            // slow path
            let key = *key;
            let change = change.clone();
            let mut remove_key = false;
            if let Some(previous_same_key_idx) = mapping.get(&key) {
              let previous_change: &mut (RawEntityHandle, ValueChange<T>) =
                changes.get_unchecked_mut(*previous_same_key_idx);
              let merge_target = &mut previous_change.1;
              if !merge_target.merge(&change) {
                remove_key = true;
                holes_indices.insert(*previous_same_key_idx);
              }
              changes.swap_remove(i);
            } else {
              mapping.insert(key, i);
              i += 1;
            }

            if remove_key {
              mapping.remove(&key);
            }
          } else {
            mapping.insert(*key, i);
            i += 1;
          }
        } else {
          break;
        }
      }

      if !holes_indices.is_empty() {
        // todo, this is bad, use swap_remove
        changes = changes
          .into_iter()
          .enumerate()
          .filter_map(|(idx, v)| {
            if holes_indices.contains(&idx) {
              None
            } else {
              Some(v)
            }
          })
          .collect();
      }
    }

    // todo, only reset what changed
    self.unique_changed_key_count = 0;
    self.has_any_change.reset();
    self.has_duplicate_changes.reset();

    FastIterQuery {
      changes: Arc::new(changes),
      mapping: Arc::new(mapping),
    }
  }
}

#[derive(Clone)]
pub struct FastIterQuery<T> {
  pub changes: Arc<Vec<(RawEntityHandle, ValueChange<T>)>>,
  pub mapping: Arc<FastHashMap<RawEntityHandle, usize>>,
}

impl<T> FastIterQuery<T> {
  pub fn is_empty(&self) -> bool {
    self.changes.is_empty()
  }
}

impl<T: CValue> Query for FastIterQuery<T> {
  type Key = RawEntityHandle;
  type Value = ValueChange<T>;

  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self.changes.iter().cloned()
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    let index = self.mapping.get(key)?;
    unsafe { self.changes.get_unchecked(*index).1.clone().into() }
  }

  fn has_item_hint(&self) -> bool {
    !self.changes.is_empty()
  }
}

impl<T> FastIterQuery<T> {
  pub fn empty() -> Self {
    Self {
      changes: Default::default(),
      mapping: Default::default(),
    }
  }
}
