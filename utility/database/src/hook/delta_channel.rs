use crate::*;

type MutationData<T> = FastDeltaChangeCollector<T>;

/// this should be a cheaper version of collective_channel
/// todo, improve code sharing with other channel
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
  pub fn poll_impl(&self, cx: &mut Context) -> Poll<Option<FastDeltaChangeCollector<T>>> {
    self.inner.1.register(cx.waker());
    let mut changes = self.inner.0.write();

    let changes = changes.take();
    if changes.has_change() {
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
  type Item = FastDeltaChangeCollector<T>;

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
  changes: Vec<(RawEntityHandle, ValueChange<T>)>,
  override_mapping: FastHashMap<RawEntityHandle, (usize, bool)>,
}

impl<T: CValue> FastDeltaChangeCollector<T> {
  pub fn new(bitmap_init: usize, change_init: usize) -> Self {
    Self {
      has_any_change: Bitmap::with_size(bitmap_init),
      has_duplicate_changes: Bitmap::with_size(bitmap_init),
      changes: Vec::with_capacity(change_init),
      override_mapping: FastHashMap::default(),
    }
  }

  pub fn reserve(&mut self, additional: usize) {
    self.changes.reserve(additional * 2); // * 2 to avoid reallocation for delta merge case
    self.has_any_change.reserve(additional);
    self.has_duplicate_changes.reserve(additional);
    self.override_mapping.reserve(additional);
  }

  pub fn has_change(&self) -> bool {
    // note, as we do not do any delta merge, this may cause extra wake in some case
    !self.changes.is_empty()
  }

  pub fn update_delta(&mut self, idx: RawEntityHandle, change: ValueChange<T>) {
    let index = idx.index() as usize;
    self.has_any_change.check_grow(index);
    self.has_duplicate_changes.check_grow(index);
    unsafe {
      let has_any_change = self.has_any_change.get(index);
      if has_any_change {
        // slow path, do hashing and maybe delta merge
        self.has_duplicate_changes.set(index, true);
        if let Some((previous_idx, has_delta)) = self.override_mapping.get_mut(&idx) {
          let previous_change: &mut (RawEntityHandle, ValueChange<T>) =
            self.changes.get_unchecked_mut(*previous_idx);

          let merge_target = &mut previous_change.1;
          if *has_delta {
            *merge_target = change;
          } else if !merge_target.merge(&change) {
            *has_delta = false;
          }
        } else {
          let change_index = self.changes.len();
          self.changes.push((idx, change));
          self.override_mapping.insert(idx, (change_index, true));
        }
      } else {
        self.has_any_change.set(index, true);
        self.changes.push((idx, change));
      }
    }
  }

  pub fn take(&mut self) -> Self {
    if !self.has_change() {
      return Self {
        has_any_change: Bitmap::with_size(0),
        has_duplicate_changes: Bitmap::with_size(0),
        changes: Vec::new(),
        override_mapping: FastHashMap::default(),
      };
    }

    let changes = std::mem::take(&mut self.changes);
    let override_mapping = std::mem::take(&mut self.override_mapping);

    // not calling std::take for these bitmaps to preserve allocation
    let has_any_change = self.has_any_change.clone();
    let has_duplicate_changes = self.has_duplicate_changes.clone();

    Self {
      has_any_change,
      has_duplicate_changes,
      changes,
      override_mapping,
    }
  }

  pub fn compute_query(self) -> FastIterQuery<T> {
    if self.changes.is_empty() {
      return FastIterQuery::empty();
    }

    let mut mapping =
      FastHashMap::with_capacity_and_hasher(self.changes.len(), FastHasherBuilder::default());

    let changes = self.changes;
    let override_mapping = self.override_mapping;

    let mut compacted_changes = Vec::with_capacity(changes.len() - override_mapping.len());

    for (change_idx, (key, change)) in changes.iter().enumerate() {
      if unsafe { !self.has_duplicate_changes.get(key.index() as usize) } {
        let i = compacted_changes.len();
        compacted_changes.push((*key, change.clone()));
        mapping.insert(*key, i);
      } else {
        let (idx, has_change) = unsafe { override_mapping.get(key).unwrap_unchecked() };
        if *idx != change_idx && *has_change {
          let (_key, override_change) = unsafe { changes.get_unchecked(*idx) };
          debug_assert_eq!(_key, key);
          let mut change_to_merge = change.clone();
          change_to_merge.merge(override_change);

          let i = compacted_changes.len();
          compacted_changes.push((*key, change_to_merge));
          mapping.insert(*key, i);
        }
      }
    }

    // assert no reallocation
    debug_assert!(compacted_changes.len() <= compacted_changes.capacity());
    debug_assert!(mapping.len() <= mapping.capacity());

    FastIterQuery {
      changes: Arc::new(compacted_changes),
      mapping: Arc::new(mapping),
    }
  }
}

#[test]
fn test() {
  fn make_handle(idx: usize) -> RawEntityHandle {
    RawEntityHandle::create_only_for_testing(idx)
  }

  let mut collector: FastDeltaChangeCollector<u32> = FastDeltaChangeCollector::new(0, 0);
  assert!(!collector.has_change());

  collector.update_delta(make_handle(3), ValueChange::Delta(1, None));
  collector.update_delta(make_handle(4), ValueChange::Delta(1, None));
  collector.update_delta(make_handle(4), ValueChange::Delta(2, Some(1)));
  collector.update_delta(make_handle(4), ValueChange::Delta(4, Some(2)));

  collector.update_delta(make_handle(5), ValueChange::Delta(2, None));
  collector.update_delta(make_handle(5), ValueChange::Remove(2));

  assert_eq!(collector.changes.len(), 5);
  assert!(collector.has_change());

  let r = collector.take().compute_query();

  assert!(!r.is_empty());

  assert_eq!(r.mapping.len(), 3);
  assert_eq!(r.changes.len(), 3);
  let v = r.access(&make_handle(3)).unwrap();
  assert_eq!(v, ValueChange::Delta(1, None));

  let v = r.access(&make_handle(4)).unwrap();
  assert_eq!(v, ValueChange::Delta(4, None));
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
