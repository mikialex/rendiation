use std::fmt::Debug;

use crate::*;

/// Black-box validator for component storage implementations.
///
/// Exercises the full [`ComponentStorage`] lifecycle through the type-erased
/// trait API, verifying correct behaviour for all operations and common edge
/// cases. `T` is the concrete component data type used for the test.
///
/// `test_values` must contain at least two distinct values, and `default_value`
/// must match [`ComponentSemantic::default_override`].
pub fn validate_component_storage<T, S>(storage: S, test_values: &[T], default_value: &T)
where
  T: DataBaseDataType + PartialEq + Debug + Clone,
  S: ComponentStorage,
{
  assert!(
    test_values.len() >= 2,
    "need at least two distinct test values"
  );

  let meta = storage.create_meta();

  // Meta integrity
  assert_eq!(
    meta.data_type_id,
    TypeId::of::<T>(),
    "data_type_id must match T"
  );
  assert_eq!(meta.shape, T::shape(), "shape must match T");

  // Helpers for converting between &T and DataPtr.
  let read = |ptr: DataPtr| -> T { unsafe { (*(ptr as *const T)).clone() } };
  let ptr_from = |val: &T| -> DataPtr { val as *const T as DataPtr };

  // Mutation phase — write view is scoped so the write lock is released
  // before any read-view creation below.  All reads during this phase go
  // through the write view's own `get()`; creating a separate read view
  // while the write lock is held would deadlock (parking_lot::RwLock does
  // not support recursive locking).
  let mut write_box = storage.create_read_write_view();
  let w: &mut dyn ComponentStorageReadWriteView = &mut *write_box;

  // resize + set_value_init round-trip
  unsafe {
    w.resize(5);
  }

  let first = &test_values[0];
  let second = &test_values[1];

  let init_ptr = unsafe { w.set_value_init(0, Some(ptr_from(first))) };
  assert_eq!(
    read(init_ptr),
    *first,
    "init should store the value and return a pointer to it"
  );

  unsafe {
    let got = read(w.get(0));
    assert_eq!(
      got, *first,
      "write_view::get should see the initialized value"
    );
  }

  // set_value (overwrite)
  let (new_ptr, old_ptr, changed) = unsafe { w.set_value(0, ptr_from(second)) };
  assert!(changed, "overwrite with different value → changed = true");
  assert_eq!(
    read(new_ptr),
    *second,
    "new_ptr should point to the new value in storage"
  );
  assert_eq!(
    read(old_ptr),
    *first,
    "old_ptr should point to the replaced value"
  );

  unsafe {
    let got = read(w.get(0));
    assert_eq!(
      got, *second,
      "write_view::get should see the overwritten value"
    );
  }

  // Idempotent write — changed = false
  let (_, _, changed_same) = unsafe { w.set_value(0, ptr_from(second)) };
  assert!(!changed_same, "writing the same value → changed = false");

  // Init with default (None path)
  unsafe {
    w.resize(6);
    let ptr = w.set_value_init(5, None);
    assert_eq!(
      read(ptr),
      *default_value,
      "init with None should set the default value"
    );
  }

  // Multiple independent slots
  unsafe {
    for (i, val) in test_values.iter().enumerate() {
      let idx = i as u32;
      if idx == 0 || idx > 4 {
        continue;
      }
      w.resize(idx);
      let ptr = w.set_value_init(idx, Some(ptr_from(val)));
      assert_eq!(
        read(ptr),
        *val,
        "slot {} should hold the written value",
        idx
      );
    }
  }

  // Delete + re-init
  unsafe {
    let old = w.delete(1);
    assert_eq!(
      read(old),
      test_values[1],
      "delete should return the old value"
    );

    let rebound = &test_values[0];
    let ptr = w.set_value_init(1, Some(ptr_from(rebound)));
    assert_eq!(read(ptr), *rebound, "slot should be reusable after delete");
  }

  // new_ptr points into storage (not caller tempo)
  unsafe {
    let val_a = &test_values[0];
    let val_b = &test_values[1];

    let (new_ptr_a, _, _) = w.set_value(2, ptr_from(val_a));
    assert_eq!(read(new_ptr_a), *val_a, "new_ptr should point to value A");

    let (new_ptr_b, old_ptr_b, _) = w.set_value(2, ptr_from(val_b));
    assert_eq!(read(new_ptr_b), *val_b, "new_ptr should point to value B");
    assert_eq!(
      read(old_ptr_b),
      *val_a,
      "old_ptr should point to value A (held in scratch)"
    );
  }

  // cleanup should not crash
  w.cleanup_possible_old_ptr_transient_object();

  // write_box drops here → write lock released
  drop(write_box);

  // Read-phase — no write lock held, read views can be created safely.

  // Read-view reads committed data
  {
    let read_view = storage.create_read_view();
    assert!(!read_view.is_heap(), "read view should be stack-allocated");

    unsafe {
      let got = read(read_view.get(0));
      assert_eq!(got, *second, "read view should see slot 0 after all writes");
    }

    let dyn_ref = unsafe { read_view.get_as_dyn_storage(0) };
    assert!(
      !dyn_ref.debug_value().is_empty(),
      "dyn debug_value() should produce output"
    );
  }

  // Write view is also stack-allocated
  {
    let wv = storage.create_read_write_view();
    assert!(!wv.is_heap(), "write view should be stack-allocated");
  }

  // memory_usage_in_bytes is positive
  assert!(
    storage.memory_usage_in_bytes() > 0,
    "memory_usage should report positive bytes"
  );
}

#[cfg(test)]
mod tests {
  use super::*;

  declare_entity!(TestEntity);
  declare_component!(TestComp, TestEntity, u32);
  declare_entity!(SparseEntity);
  declare_component!(SparseComp, SparseEntity, u32);

  #[test]
  fn validate_linear_storage() {
    let storage = init_linear_storage::<TestComp>();
    validate_component_storage::<u32, _>(storage, &[42, 99, 7, 1234], &0u32);
  }

  #[test]
  fn validate_sparse_storage() {
    let storage = init_sparse_storage::<SparseComp>();
    validate_component_storage::<u32, _>(storage, &[42, 99, 7, 1234], &0u32);
  }
}
