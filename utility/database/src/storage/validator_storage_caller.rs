use std::sync::Mutex;

use crate::*;

#[derive(Clone, Copy, PartialEq, Debug)]
enum SlotState {
  Vacant,
  Occupied,
}

struct SlotTracker {
  states: Vec<SlotState>,
}

impl SlotTracker {
  fn new() -> Self {
    SlotTracker { states: Vec::new() }
  }

  fn handle_resize(&mut self, max_address: u32) {
    if self.states.len() <= max_address as usize {
      self
        .states
        .resize(max_address as usize + 1, SlotState::Vacant);
    }
  }

  fn check_init(&self, idx: u32) {
    assert!(
      (idx as usize) < self.states.len(),
      "callee-violation: set_value_init(idx = {idx}) before resize({})",
      self.states.len().saturating_sub(1),
    );
    assert!(
      self.states[idx as usize] == SlotState::Vacant,
      "callee-violation: set_value_init(idx = {idx}) on already-occupied slot \
       (previous occupant was not deleted)",
    );
  }

  fn check_write(&self, idx: u32) {
    assert!(
      (idx as usize) < self.states.len(),
      "callee-violation: set_value(idx = {idx}) before resize({})",
      self.states.len().saturating_sub(1),
    );
    assert!(
      self.states[idx as usize] == SlotState::Occupied,
      "callee-violation: set_value(idx = {idx}) on non-living slot \
       (call set_value_init first)",
    );
  }

  fn mark_occupied(&mut self, idx: u32) {
    self.states[idx as usize] = SlotState::Occupied;
  }

  fn mark_vacant(&mut self, idx: u32) {
    self.states[idx as usize] = SlotState::Vacant;
  }

  fn check_delete(&self, idx: u32) {
    assert!(
      (idx as usize) < self.states.len(),
      "callee-violation: delete(idx = {idx}) before resize({})",
      self.states.len().saturating_sub(1),
    );
    assert!(
      self.states[idx as usize] == SlotState::Occupied,
      "callee-violation: delete(idx = {idx}) on non-living slot \
       (not initialized or already deleted)",
    );
  }

  fn memory_usage(&self) -> usize {
    self.states.capacity() * std::mem::size_of::<SlotState>()
  }
}

struct ValidatedWriteView {
  inner: ComponentReadWriteViewBox,
  slots: Arc<Mutex<SlotTracker>>,
}

impl ComponentStorageReadViewBase for ValidatedWriteView {
  fn meta(&self) -> &DataTypeMetaInfo {
    self.inner.meta()
  }

  unsafe fn get(&self, idx: u32) -> DataPtr {
    self.inner.get(idx)
  }
}

impl ComponentStorageReadWriteView for ValidatedWriteView {
  unsafe fn set_value_init(&mut self, idx: u32, init_value: Option<DataPtr>) -> DataPtr {
    let mut guard = self.slots.lock().unwrap();
    guard.check_init(idx);
    let result = self.inner.set_value_init(idx, init_value);
    guard.mark_occupied(idx);
    result
  }

  unsafe fn set_value(&mut self, idx: u32, new_value: DataPtr) -> (DataPtr, DataPtr, bool) {
    self.slots.lock().unwrap().check_write(idx);
    self.inner.set_value(idx, new_value)
  }

  unsafe fn set_value_from_serialize_field_data(
    &mut self,
    idx: u32,
    new_value: DatabaseSerializedFieldBufferOrForeignKey,
  ) -> (DataPtr, DataPtr, bool) {
    self.slots.lock().unwrap().check_write(idx);
    self
      .inner
      .set_value_from_serialize_field_data(idx, new_value)
  }

  unsafe fn delete(&mut self, idx: u32) -> DataPtr {
    let mut guard = self.slots.lock().unwrap();
    guard.check_delete(idx);
    let result = self.inner.delete(idx);
    guard.mark_vacant(idx);
    result
  }

  unsafe fn resize(&mut self, max_address: u32) {
    <dyn ComponentStorageReadWriteView>::resize(&mut *self.inner, max_address);
    self.slots.lock().unwrap().handle_resize(max_address);
  }

  fn cleanup_possible_old_ptr_transient_object(&mut self) {
    self.inner.cleanup_possible_old_ptr_transient_object()
  }
}

/// A [`ComponentStorage`] wrapper that validates whether callers adhere to
/// the [`ComponentStorageReadWriteView`] protocol constraints.
///
/// Wrap any existing storage with [`ValidatedStorage::new`] to enable
/// runtime assertion checking.  When a violation is detected the operation
/// panics with a message describing the problem.
#[derive(Clone)]
pub struct ValidatedStorage {
  inner: Box<dyn ComponentStorage>,
  slots: Arc<Mutex<SlotTracker>>,
}

impl ValidatedStorage {
  pub fn new(inner: Box<dyn ComponentStorage>) -> Self {
    ValidatedStorage {
      inner,
      slots: Arc::new(Mutex::new(SlotTracker::new())),
    }
  }
}

impl ComponentStorage for ValidatedStorage {
  fn create_read_view(&self) -> ComponentReadViewBox {
    self.inner.create_read_view()
  }

  fn create_read_write_view(&self) -> ComponentReadWriteViewBox {
    smallbox!(ValidatedWriteView {
      inner: self.inner.create_read_write_view(),
      slots: self.slots.clone(),
    })
  }

  fn create_meta(&self) -> DataTypeMetaInfo {
    self.inner.create_meta()
  }

  fn memory_usage_in_bytes(&self) -> usize {
    let tracker_mem = self.slots.lock().unwrap().memory_usage();
    self.inner.memory_usage_in_bytes() + tracker_mem
  }
}
