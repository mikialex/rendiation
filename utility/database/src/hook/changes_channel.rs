use crate::*;

type MutationData<T> = FastChangeCollector<T>;

/// this should be a cheaper version of collective_channel
///  improve code sharing with other channel
pub fn changes_channel<T>(
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
  pub unsafe fn send(&self, idx: u32, change: Option<T>) {
    let mutations = &mut *self.inner.0.data_ptr();

    mutations.update_change(idx, change);
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
  pub fn poll_impl(&self, cx: &mut Context) -> Poll<Option<MutationData<T>>> {
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
  type Item = MutationData<T>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.poll_impl(cx)
  }
}

pub(crate) fn add_changes_listen<T: CValue>(
  bitmap_init: usize,
  query: impl Query<Key = RawEntityHandle, Value = T>,
  source: &EventSource<ChangePtr>,
) -> ChangesMutationReceiver<T> {
  let (sender, receiver) = changes_channel::<T>(bitmap_init, 0);
  // expand initial value while first listen.
  unsafe {
    sender.lock();
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

/// the optimization assumes: between the updates, one component is only changed once
/// in this case, this collector can avoid any key hash operation.
#[derive(Clone)]
pub struct FastChangeCollector<T> {
  removed_set: Bitmap,
  inserted_set: Bitmap,
  inserted_override_set: Bitmap,
  removed_keys: Vec<u32>,
  new_or_inserts: Vec<(u32, T)>,
  override_new_or_inserts: Vec<(u32, T)>,
  override_mapping: FastHashMap<u32, usize>,
}

impl<T> FastChangeCollector<T> {
  pub fn empty() -> Self {
    Self {
      removed_set: Bitmap::with_size(0),
      inserted_set: Bitmap::with_size(0),
      inserted_override_set: Bitmap::with_size(0),
      removed_keys: Vec::new(),
      new_or_inserts: Vec::new(),
      override_new_or_inserts: Vec::new(),
      override_mapping: FastHashMap::default(),
    }
  }

  // todo: note the bitmap's size will always increase, this may becomes a problem.
  pub fn take(&mut self) -> Self {
    let removed_keys = std::mem::take(&mut self.removed_keys);
    let new_or_inserts = std::mem::take(&mut self.new_or_inserts);
    let override_new_or_inserts = std::mem::take(&mut self.override_new_or_inserts);

    if removed_keys.is_empty() && new_or_inserts.is_empty() && override_new_or_inserts.is_empty() {
      assert!(self.override_mapping.is_empty());
      return Self::empty();
    }

    let override_mapping = std::mem::take(&mut self.override_mapping);
    let removed_set = self.removed_set.clone();
    let inserted_set = self.inserted_set.clone();
    let inserted_override_set = self.inserted_override_set.clone();

    // todo, only reset what changed
    self.removed_set.reset();
    self.inserted_override_set.reset();
    self.inserted_set.reset();

    Self {
      inserted_set,
      removed_set,
      inserted_override_set,
      removed_keys,
      new_or_inserts,
      override_new_or_inserts,
      override_mapping,
    }
  }

  pub fn new(bitmap_init: usize, change_init: usize) -> Self {
    Self {
      inserted_set: Bitmap::with_size(bitmap_init),
      removed_set: Bitmap::with_size(bitmap_init),
      inserted_override_set: Bitmap::with_size(bitmap_init),
      removed_keys: Vec::with_capacity(change_init),
      new_or_inserts: Vec::with_capacity(change_init),
      override_new_or_inserts: Vec::new(),
      override_mapping: FastHashMap::default(),
    }
  }

  pub fn reserve(&mut self, additional: usize) {
    self.removed_keys.reserve(additional);
    self.new_or_inserts.reserve(additional);
    self.removed_set.reserve(additional);
    self.inserted_set.reserve(additional);
    self.inserted_override_set.reserve(additional);
  }

  pub fn update_change(&mut self, idx: u32, change: Option<T>) {
    let idx_usize = idx as usize;
    self.removed_set.check_grow(idx_usize);
    self.inserted_override_set.check_grow(idx_usize);
    self.inserted_set.check_grow(idx_usize);

    unsafe {
      if let Some(change) = change {
        self.removed_set.set(idx_usize, false);

        if self.inserted_set.get(idx_usize) {
          self.update_new_slow_path(idx, change);
        } else {
          self.inserted_set.set(idx_usize, true);
          self.new_or_inserts.push((idx, change));
        }
      } else {
        self.removed_keys.push(idx);
        self.removed_set.set(idx_usize, true);
      }
    }
  }

  fn update_new_slow_path(&mut self, idx: u32, change: T) {
    if let Some(previous_override_idx) = self.override_mapping.get(&idx) {
      unsafe {
        *self
          .override_new_or_inserts
          .get_unchecked_mut(*previous_override_idx) = (idx, change);
      }
    } else {
      self.override_new_or_inserts.push((idx, change));
      self
        .override_mapping
        .insert(idx, self.override_new_or_inserts.len() - 1);
      unsafe {
        self.inserted_override_set.set(idx as usize, true);
      }
    }
  }
}

impl<T: CValue> DataChanges for FastChangeCollector<T> {
  type Key = u32;
  type Value = T;

  fn has_change(&self) -> bool {
    !(self.removed_keys.is_empty()
      && self.new_or_inserts.is_empty()
      && self.override_new_or_inserts.is_empty())
  }

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self
      .removed_keys
      .iter()
      .copied()
      .filter(|idx| unsafe { self.removed_set.get(*idx as usize) })
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    let new_or_inserts = self.new_or_inserts.iter().filter_map(|(idx, v)| unsafe {
      if self.removed_set.get(*idx as usize) {
        return None;
      };

      if self.inserted_override_set.get(*idx as usize) {
        return None;
      };

      Some((*idx, v.clone()))
    });

    let override_new_or_inserts =
      self
        .override_new_or_inserts
        .iter()
        .filter_map(|(idx, v)| unsafe {
          if self.removed_set.get(*idx as usize) {
            return None;
          };

          Some((*idx, v.clone()))
        });

    new_or_inserts.chain(override_new_or_inserts)
  }
}

#[derive(Clone)]
pub struct Bitmap {
  bits: Vec<u8>,
}

impl Bitmap {
  pub fn empty() -> Self {
    Self { bits: vec![] }
  }
  /// Create a new bitmap with initial size `size`
  pub fn with_size(size: usize) -> Self {
    let byte_size = (size >> 3) + 1;
    Self {
      bits: vec![0; byte_size],
    }
  }

  pub fn reserve(&mut self, additional: usize) {
    let new_len = self.bits.len() * 8 + additional;
    self.check_grow(new_len);
  }

  pub fn reset(&mut self) {
    self.bits.fill(0);
  }

  #[inline(always)]
  pub fn check_grow(&mut self, at_least_new_size: usize) {
    let byte_size = (at_least_new_size >> 3) + 1;
    let byte_size = byte_size.max(self.bits.len());
    self.bits.resize(byte_size, 0);
  }

  /// # Safety
  ///
  /// idx must in bound
  #[inline(always)]
  pub unsafe fn get(&self, idx: usize) -> bool {
    let byte_idx = idx >> 3; // idx / 8
    let offset = idx & 0b111; // idx % 8
    let byte = self.bits.get_unchecked(byte_idx);
    (byte >> (7 - offset)) & 1 == 1
  }

  /// # Safety
  ///
  /// idx must in bound
  #[inline(always)]
  pub unsafe fn set(&mut self, idx: usize, value: bool) {
    let byte_idx = idx >> 3; // idx / 8
    let offset = idx & 0b111; // idx % 8

    let byte_ref = self.bits.get_unchecked_mut(byte_idx);

    let byte = *byte_ref;

    let curval = (byte >> (7 - offset)) & 1;
    let mask = if value { 1 ^ curval } else { curval };

    *byte_ref = byte ^ (mask << (7 - offset)); // Bit flipping
  }
}
