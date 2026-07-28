use super::*;

#[test]
fn empty_update_returns_none() {
  let mut reducer: HierarchyMonoidReducer<&str, i32> = HierarchyMonoidReducer::default();
  assert_eq!(reducer.update(|a, b| a + b), None);
}

#[test]
fn single_insert_update() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 42);
  assert_eq!(reducer.update(|a, b| a + b), Some(42));
}

#[test]
fn multiple_inserts_sum() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 1);
  reducer.notify_insert_or_update("b", 2);
  reducer.notify_insert_or_update("c", 3);
  assert_eq!(reducer.update(|a, b| a + b), Some(6));
}

#[test]
fn multiple_inserts_max() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 5);
  reducer.notify_insert_or_update("b", 10);
  reducer.notify_insert_or_update("c", 3);
  reducer.notify_insert_or_update("d", 8);
  assert_eq!(reducer.update(|a, b| a.max(b)), Some(10));
}

#[test]
fn multiple_inserts_min() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 5);
  reducer.notify_insert_or_update("b", 1);
  reducer.notify_insert_or_update("c", 3);
  assert_eq!(reducer.update(|a, b| a.min(b)), Some(1));
}

#[test]
fn update_existing_key() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 10);
  assert_eq!(reducer.update(|a, b| a + b), Some(10));

  reducer.notify_insert_or_update("a", 20);
  assert_eq!(reducer.update(|a, b| a + b), Some(20));
}

#[test]
fn update_existing_key_with_others() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 1);
  reducer.notify_insert_or_update("b", 2);
  reducer.notify_insert_or_update("c", 3);
  assert_eq!(reducer.update(|a, b| a + b), Some(6));

  reducer.notify_insert_or_update("b", 20);
  assert_eq!(reducer.update(|a, b| a + b), Some(24)); // 1 + 20 + 3
}

#[test]
fn remove_existing() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 1);
  reducer.notify_insert_or_update("b", 2);
  reducer.notify_insert_or_update("c", 3);
  assert_eq!(reducer.update(|a, b| a + b), Some(6));

  reducer.notify_remove(&"b");
  assert_eq!(reducer.update(|a, b| a + b), Some(4)); // 1 + 3

  reducer.notify_remove(&"a");
  assert_eq!(reducer.update(|a, b| a + b), Some(3)); // just c

  reducer.notify_remove(&"c");
  assert_eq!(reducer.update(|a, b| a + b), None);
}

#[test]
fn remove_nonexistent_is_noop() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 1);
  reducer.notify_remove(&"b"); // no-op
  assert_eq!(reducer.update(|a, b| a + b), Some(1));
}

#[test]
fn swap_remove_moves_last_item() {
  // insert 4 items, remove a middle one, verify the last item was swapped in
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 1);
  reducer.notify_insert_or_update("b", 2);
  reducer.notify_insert_or_update("c", 3);
  reducer.notify_insert_or_update("d", 4);
  assert_eq!(reducer.update(|a, b| a + b), Some(10));

  // remove "b" (middle), "d" should be swapped into b's slot
  reducer.notify_remove(&"b");
  // after swap-remove: a=1, d=4, c=3
  assert_eq!(reducer.update(|a, b| a + b), Some(8)); // 1 + 4 + 3

  // verify "d" can still be removed (mapping was updated correctly)
  reducer.notify_remove(&"d");
  assert_eq!(reducer.update(|a, b| a + b), Some(4)); // 1 + 3
}

#[test]
fn swap_remove_last_item() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 1);
  reducer.notify_insert_or_update("b", 2);
  reducer.notify_insert_or_update("c", 3);
  assert_eq!(reducer.update(|a, b| a + b), Some(6));

  // remove last item, no swap needed
  reducer.notify_remove(&"c");
  assert_eq!(reducer.update(|a, b| a + b), Some(3)); // 1 + 2
}

#[test]
fn insert_after_remove_reuses_slot() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 1);
  reducer.notify_insert_or_update("b", 2);
  reducer.notify_insert_or_update("c", 3);
  assert_eq!(reducer.update(|a, b| a + b), Some(6));

  reducer.notify_remove(&"b");
  assert_eq!(reducer.update(|a, b| a + b), Some(4)); // 1 + 3 (swap: a=1, c=3)

  // insert new item
  reducer.notify_insert_or_update("d", 10);
  assert_eq!(reducer.update(|a, b| a + b), Some(14)); // 1 + 3 + 10
}

#[test]
fn grow_triggered_by_insert() {
  let mut reducer = HierarchyMonoidReducer::default(); // pot = 1

  // pot=1, first insert fits
  reducer.notify_insert_or_update("a", 1);
  assert_eq!(reducer.update(|a, b| a + b), Some(1));

  // pot=1, count=1, second insert triggers grow to pot=2
  reducer.notify_insert_or_update("b", 2);
  assert_eq!(reducer.update(|a, b| a + b), Some(3));

  // pot=2, count=2, third triggers grow to pot=4
  reducer.notify_insert_or_update("c", 3);
  assert_eq!(reducer.update(|a, b| a + b), Some(6));

  // pot=4, fill it
  reducer.notify_insert_or_update("d", 4);
  assert_eq!(reducer.update(|a, b| a + b), Some(10)); // 1+2+3+4

  // pot=4, count=4, this triggers grow to pot=8
  reducer.notify_insert_or_update("e", 5);
  assert_eq!(reducer.update(|a, b| a + b), Some(15)); // 1+2+3+4+5
}

#[test]
fn shrink_triggered_by_remove() {
  let mut reducer = HierarchyMonoidReducer::default();

  // insert 5 items: pot grows to 8
  reducer.notify_insert_or_update("a", 1);
  reducer.notify_insert_or_update("b", 2);
  reducer.notify_insert_or_update("c", 3);
  reducer.notify_insert_or_update("d", 4);
  reducer.notify_insert_or_update("e", 5);
  assert_eq!(reducer.pot, 8);
  assert_eq!(reducer.update(|a, b| a + b), Some(15));

  // remove down to count=3, which < pot/2=4, triggers shrink to pot=4
  reducer.notify_remove(&"e");
  reducer.notify_remove(&"d");
  assert_eq!(reducer.update(|a, b| a + b), Some(6)); // 1+2+3
  assert_eq!(reducer.pot, 4);

  // remove down to count=1, which < pot/2=2, triggers shrink to pot=2
  reducer.notify_remove(&"c");
  reducer.notify_remove(&"b");
  assert_eq!(reducer.update(|a, b| a + b), Some(1));
  assert_eq!(reducer.pot, 2);
}

#[test]
fn shrink_preserves_internals_correctly() {
  let mut reducer = HierarchyMonoidReducer::default();

  // build a tree with 9 items (pot=16)
  let mut sum = 0;
  for i in 0..9 {
    let key = format!("k{i}");
    reducer.notify_insert_or_update(key, (i + 1) as i32);
    sum += (i + 1) as i32;
  }
  assert_eq!(reducer.update(|a, b| a + b), Some(sum)); // 45

  // remove 6 items, remaining 3 items < 16/2=8, triggers shrink to pot=8
  for i in 3..9 {
    let key = format!("k{i}");
    reducer.notify_remove(&key);
  }
  let remaining: i32 = (1..=3).sum(); // 6
  assert_eq!(reducer.update(|a, b| a + b), Some(remaining));
  assert!(reducer.pot <= 8); // may have shrunk
}

#[test]
fn multiple_updates_without_notify_return_same() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 10);
  reducer.notify_insert_or_update("b", 20);

  assert_eq!(reducer.update(|a, b| a + b), Some(30));
  assert_eq!(reducer.update(|a, b| a + b), Some(30));
  assert_eq!(reducer.update(|a, b| a + b), Some(30));
}

#[test]
fn dirty_dedup_multiple_notifies_same_key() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 1);
  reducer.notify_insert_or_update("a", 2);
  reducer.notify_insert_or_update("a", 3);
  // dirty_flags prevents duplicate entries in dirty_indices
  assert_eq!(reducer.update(|a, b| a + b), Some(3));
}

#[test]
fn interleaved_insert_remove_update() {
  let mut reducer = HierarchyMonoidReducer::default();

  reducer.notify_insert_or_update("a", 1);
  reducer.notify_insert_or_update("b", 2);
  assert_eq!(reducer.update(|a, b| a + b), Some(3));

  reducer.notify_remove(&"a");
  reducer.notify_insert_or_update("c", 5);
  assert_eq!(reducer.update(|a, b| a + b), Some(7)); // 2 + 5

  reducer.notify_insert_or_update("b", 10);
  reducer.notify_insert_or_update("d", 3);
  assert_eq!(reducer.update(|a, b| a + b), Some(18)); // 10 + 5 + 3
}

#[test]
fn large_insert_stress() {
  let mut reducer = HierarchyMonoidReducer::default();
  let n = 100;

  for i in 0..n {
    reducer.notify_insert_or_update(i, i as i64);
  }

  let expected_sum: i64 = (0..n).sum();
  assert_eq!(reducer.update(|a, b| a + b), Some(expected_sum));

  // remove even numbers
  for i in (0..n).step_by(2) {
    reducer.notify_remove(&i);
  }

  let expected_odd_sum: i64 = (1..n).step_by(2).sum();
  assert_eq!(reducer.update(|a, b| a + b), Some(expected_odd_sum));
}

#[test]
fn string_concat_as_monoid() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", "hello".to_string());
  reducer.notify_insert_or_update("b", " ".to_string());
  reducer.notify_insert_or_update("c", "world".to_string());

  assert_eq!(
    reducer.update(|a, b| format!("{a}{b}")),
    Some("hello world".to_string())
  );
}

#[test]
fn pot_starts_at_one() {
  let reducer: HierarchyMonoidReducer<&str, i32> = HierarchyMonoidReducer::default();
  assert_eq!(reducer.pot, 1);
  assert_eq!(reducer.count, 0);
}

#[test]
fn add_remove_add_same_key() {
  let mut reducer = HierarchyMonoidReducer::default();
  reducer.notify_insert_or_update("a", 10);
  assert_eq!(reducer.update(|a, b| a + b), Some(10));

  reducer.notify_remove(&"a");
  assert_eq!(reducer.update(|a, b| a + b), None);

  reducer.notify_insert_or_update("a", 20);
  assert_eq!(reducer.update(|a, b| a + b), Some(20));
}
