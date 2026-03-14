use crate::*;

type MutationData<T> = FastDeltaChangeCollector<T>;

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
  query: impl Query<Key = RawEntityHandle, Value = T>,
  source: &EventSource<ChangePtr>,
) -> ChangesMutationReceiver<T> {
  let (sender, receiver) = delta_channel::<T>(bitmap_init, 0);
  // expand initial value while first listen.
  unsafe {
    sender.lock();
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
#[derive(Clone, Debug)]
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
    !self.changes.is_empty()
  }

  pub fn update_delta(&mut self, handle: RawEntityHandle, change: ValueChange<T>) {
    let index = handle.index() as usize;
    self.has_any_change.check_grow(index);
    self.has_duplicate_changes.check_grow(index);
    unsafe {
      let has_any_change = self.has_any_change.get(index);
      if has_any_change {
        // slow path, do hashing and maybe delta merge
        self.has_duplicate_changes.set(index, true);
        if let Some((the_second_change_idx, has_delta)) = self.override_mapping.get_mut(&handle) {
          let the_second_change: &mut (RawEntityHandle, ValueChange<T>) =
            self.changes.get_unchecked_mut(*the_second_change_idx);

          if *has_delta {
            let merge_target = &mut the_second_change.1;
            if !merge_target.merge(&change) {
              *has_delta = false;
            }
          } else {
            *has_delta = true;
            the_second_change.1 = change;
          }
        } else {
          let change_index = self.changes.len();
          self.changes.push((handle, change));
          self.override_mapping.insert(handle, (change_index, true));
        }
      } else {
        self.has_any_change.set(index, true);
        self.changes.push((handle, change));
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

    // todo, only reset changed
    self.has_any_change.reset();
    self.has_duplicate_changes.reset();

    Self {
      has_any_change,
      has_duplicate_changes,
      changes,
      override_mapping,
    }
  }

  pub fn is_empty(&self) -> bool {
    self.changes.is_empty()
  }

  pub fn compute_query(self) -> DBDelta<T> {
    let mut mapping = FastHashMap::with_capacity_and_hasher(self.changes.len(), Default::default());

    let changes = self.changes;
    let override_mapping = self.override_mapping;

    let mut processed_override_second_change_index: FastHashSet<usize> =
      FastHashSet::with_capacity_and_hasher(override_mapping.len(), Default::default());

    for (current_change_idx, (handle, change)) in changes.iter().enumerate() {
      if unsafe { !self.has_duplicate_changes.get(handle.index() as usize) } {
        mapping.insert(*handle, change.clone());
      } else {
        if let Some((the_second_change_index, has_change)) = override_mapping.get(handle) {
          if processed_override_second_change_index.insert(*the_second_change_index) {
            // not processed case
            if *the_second_change_index == current_change_idx {
              mapping.insert(*handle, change.clone());
            } else {
              if *has_change {
                let (_key, override_change) =
                  unsafe { changes.get_unchecked(*the_second_change_index) };
                debug_assert_eq!(_key, handle);
                let mut change_to_merge = change.clone();
                if change_to_merge.merge(override_change) {
                  mapping.insert(*handle, change_to_merge);
                }
              }
            }
          }
        } else {
          // this is possible, because the access key may be a slate handle(but in same position, so it passed bitset check).
          debug_assert!(change.is_removed());
          mapping.insert(*handle, change.clone());
        }
      }
    }

    // assert no reallocation
    debug_assert!(mapping.len() <= mapping.capacity());

    Arc::new(mapping)
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

  assert_eq!(r.len(), 2);
  let v = r.access(&make_handle(3)).unwrap();
  assert_eq!(v, ValueChange::Delta(1, None));

  let v = r.access(&make_handle(4)).unwrap();
  assert_eq!(v, ValueChange::Delta(4, None));
}

#[test]
fn test_same_position_add_remove_edge_case() {
  fn make_handle(idx: usize, g: u64) -> RawEntityHandle {
    RawEntityHandle::create_only_for_testing_with_gen(idx, g)
  }

  let mut collector: FastDeltaChangeCollector<u32> = FastDeltaChangeCollector::new(0, 0);
  assert!(!collector.has_change());

  collector.update_delta(make_handle(3, 0), ValueChange::Delta(1, None));
  collector.update_delta(make_handle(3, 0), ValueChange::Remove(1));
  collector.update_delta(make_handle(3, 1), ValueChange::Delta(2, None));

  assert_eq!(collector.changes.len(), 3);
  assert!(collector.has_change());

  // dbg!(&collector);

  let r = collector.take().compute_query();

  assert_eq!(r.len(), 1);
  let v = r.access(&make_handle(3, 1)).unwrap();
  assert_eq!(v, ValueChange::Delta(2, None));
}

#[test]
fn test_same_position_add_remove_edge_case2() {
  fn make_handle(idx: usize, g: u64) -> RawEntityHandle {
    RawEntityHandle::create_only_for_testing_with_gen(idx, g)
  }

  let mut collector: FastDeltaChangeCollector<u32> = FastDeltaChangeCollector::new(0, 0);
  assert!(!collector.has_change());

  collector.update_delta(make_handle(3, 0), ValueChange::Remove(1));
  collector.update_delta(make_handle(3, 1), ValueChange::Delta(2, None));

  assert_eq!(collector.changes.len(), 2);
  assert!(collector.has_change());

  // dbg!(&collector);

  let r = collector.take().compute_query();

  assert_eq!(r.len(), 2);

  let v = r.access(&make_handle(3, 0)).unwrap();
  assert_eq!(v, ValueChange::Remove(1));

  let v = r.access(&make_handle(3, 1)).unwrap();
  assert_eq!(v, ValueChange::Delta(2, None));
}
