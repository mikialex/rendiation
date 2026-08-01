use super::*;

fn new_alloc(init: u32, max: u32) -> GrowableRangeAllocator<u32> {
  GrowableRangeAllocator::new("test", max, init, 1)
}

fn assert_exclusive(r: &BatchAllocateResult<u32>) {
  for k in &r.removed {
    assert!(!r.failed_to_allocate.contains(k));
    assert!(!r.data_movements.contains_key(k));
    assert!(!r.new_data_to_write.contains_key(k));
  }
  for k in &r.failed_to_allocate {
    assert!(!r.data_movements.contains_key(k));
    assert!(!r.new_data_to_write.contains_key(k));
  }
  for k in r.data_movements.keys() {
    assert!(!r.new_data_to_write.contains_key(k));
  }
}

#[test]
fn insert_then_query_returns_region() {
  let mut alloc = new_alloc(8, 64);
  let r = alloc.update([].into_iter(), [(1, 3), (2, 5)]);
  assert_eq!(alloc.get_region(&1), Some((3, 0)));
  assert_eq!(alloc.get_region(&2), Some((5, 3)));
  assert_eq!(alloc.get_region(&3), None);
  assert_eq!(alloc.used_count, 8);
  assert_exclusive(&r);
}

#[test]
fn pure_insert_change_count_is_not_zero() {
  let mut alloc = new_alloc(8, 64);
  let r = alloc.update([].into_iter(), [(1, 2)]);
  assert_eq!(r.change_count(), 1);
  assert!(r.removed.is_empty());
  assert!(r.failed_to_allocate.is_empty());
  assert!(r.data_movements.is_empty());
  assert_eq!(r.new_data_to_write.len(), 1);
}

#[test]
fn remove_releases_range() {
  let mut alloc = new_alloc(8, 64);
  alloc.update([].into_iter(), [(1, 3)]);
  assert_eq!(alloc.used_count, 3);
  let r = alloc.update([1].into_iter(), []);
  assert!(r.removed.contains(&1));
  assert_eq!(alloc.used_count, 0);
  assert_eq!(alloc.get_region(&1), None);
  assert_exclusive(&r);
}

#[test]
fn update_existing_key_replaces_range() {
  let mut alloc = new_alloc(8, 64);
  alloc.update([].into_iter(), [(1, 3), (2, 4)]);
  let r = alloc.update([1].into_iter(), [(1, 7)]);
  assert!(!r.removed.contains(&1));
  assert_eq!(r.new_data_to_write.get(&1), Some(&(4, 7)));
  assert_eq!(alloc.get_region(&1), Some((7, 4)));
  assert_eq!(alloc.used_count, 11);
  assert_exclusive(&r);
}

#[test]
fn grow_relocates_existing_and_allocates_new() {
  let mut alloc = new_alloc(4, 64);
  alloc.update([].into_iter(), [(1, 2), (2, 2)]);
  let r = alloc.update([].into_iter(), [(3, 2)]);
  assert_eq!(r.resize_to, Some(6));
  assert_eq!(r.data_movements.len(), 2);
  assert_eq!(r.new_data_to_write.len(), 1);
  assert_eq!(alloc.get_region(&3), Some((2, 4)));
  assert_exclusive(&r);
}

#[test]
fn movement_chain_continuous_across_updates() {
  let mut alloc = new_alloc(8, 64);
  alloc.update([].into_iter(), [(1, 4), (2, 4)]);
  let r1 = alloc.update([].into_iter(), [(3, 4)]);
  assert_eq!(r1.resize_to, Some(13));
  let m1 = r1.data_movements.get(&1).unwrap();
  // each update performs a real resize, so the data lives at m1.new_offset
  // before the next update moves it
  let r2 = alloc.update([].into_iter(), [(4, 4)]);
  assert_eq!(r2.resize_to, Some(17));
  let m2 = r2.data_movements.get(&1).unwrap();
  assert_eq!(m2.old_offset, m1.new_offset);
  assert_exclusive(&r2);
}

#[test]
fn movement_merge_within_single_update() {
  let mut alloc = new_alloc(2, 8);
  alloc.update([].into_iter(), [(1, 1), (2, 1)]);
  let r = alloc.update([].into_iter(), [(3, 4), (4, 4), (5, 2)]);
  // relocate to 4 fails for key 3, then relocate to 8 succeeds,
  // movements recorded in two rounds must be merged into a single old -> new
  assert_eq!(r.resize_to, Some(8));
  let m1 = r.data_movements.get(&1).unwrap();
  let m2 = r.data_movements.get(&2).unwrap();
  assert_eq!(m1.old_offset, 0);
  assert_eq!(m2.old_offset, 1);
  assert_eq!(m1.count, 1);
  assert_eq!(m2.count, 1);
  assert!(r.failed_to_allocate.contains(&4));
  assert_eq!(alloc.get_region(&5), Some((2, 6)));
  assert_exclusive(&r);
}

#[test]
fn reach_max_fails_and_rolls_back_used_count() {
  let mut alloc = new_alloc(4, 4);
  alloc.update([].into_iter(), [(1, 2), (2, 2)]);
  let r = alloc.update([].into_iter(), [(3, 2)]);
  assert!(r.failed_to_allocate.contains(&3));
  assert_eq!(alloc.used_count, 4);
  assert_eq!(alloc.get_region(&3), None);
  assert_eq!(alloc.get_region(&1), Some((2, 0)));
  assert_exclusive(&r);
}

#[test]
fn removed_and_failed_are_exclusive() {
  let mut alloc = new_alloc(4, 4);
  alloc.update([].into_iter(), [(1, 1), (2, 3)]);
  let r = alloc.update([1].into_iter(), [(1, 4)]);
  assert!(r.failed_to_allocate.contains(&1));
  assert!(!r.removed.contains(&1));
  assert_eq!(alloc.get_region(&1), None);
  assert_eq!(alloc.used_count, 3);
  assert_exclusive(&r);
}

#[test]
fn failed_key_retry_succeeds_after_space_freed() {
  let mut alloc = new_alloc(4, 4);
  alloc.update([].into_iter(), [(1, 2), (2, 2)]);
  alloc.update([].into_iter(), [(3, 2)]);
  let r = alloc.update([2].into_iter(), [(3, 2)]);
  assert_eq!(alloc.get_region(&3), Some((2, 2)));
  assert_eq!(alloc.used_count, 4);
  assert!(r.new_data_to_write.contains_key(&3));
  assert_exclusive(&r);
}

#[test]
fn used_count_matches_ranges_after_mixed_updates() {
  let mut alloc = new_alloc(8, 32);
  alloc.update([].into_iter(), [(1, 3), (2, 5), (3, 2)]);
  alloc.update([2].into_iter(), [(2, 6), (4, 4)]);
  let r = alloc.update([1, 4].into_iter(), [(1, 4), (5, 8)]);
  let expected: u32 = alloc.ranges.values().map(|&(s, _, _)| s).sum();
  assert_eq!(alloc.used_count, expected);
  assert!(alloc.current_count >= alloc.used_count);
  for (k, &(size, offset, _)) in &alloc.ranges {
    assert_eq!(alloc.get_region(k), Some((size, offset)));
  }
  assert_exclusive(&r);
}

fn new_aligned_alloc(init: u32, max: u32) -> GrowableRangeAllocator<u32> {
  GrowableRangeAllocator::new("test", max, init, 4)
}

#[test]
fn alignment_offsets_are_aligned() {
  let mut alloc = new_aligned_alloc(16, 64);
  alloc.update([].into_iter(), [(1, 5), (2, 7), (3, 9)]);
  for k in [1, 2, 3] {
    let (_, offset) = alloc.get_region(&k).unwrap();
    assert_eq!(offset % 4, 0);
  }
}

#[test]
fn alignment_nominal_capacity_seems_enough_but_fragment() {
  // four 3-byte items aligned to 4 fill the 16 slots completely,
  // while the nominal remain capacity still reports 4
  let mut alloc = new_aligned_alloc(16, 64);
  alloc.update([].into_iter(), [(1, 3), (2, 3), (3, 3), (4, 3)]);
  let r = alloc.update([].into_iter(), [(5, 3)]);
  assert_eq!(r.resize_to, Some(32));
  assert_eq!(r.data_movements.len(), 4);
  assert_eq!(r.new_data_to_write.len(), 1);
  assert_eq!(alloc.get_region(&5), Some((3, 16)));
  assert_exclusive(&r);
}

#[test]
fn alignment_1_1_grow_factor_not_enough_falls_back_to_loop() {
  // the pre-grow with 1.1 factor (19) cannot fit the aligned footprint,
  // the loop relocate to 38 succeeds
  let mut alloc = new_aligned_alloc(16, 64);
  alloc.update([].into_iter(), [(1, 3), (2, 3), (3, 3), (4, 3)]);
  let r = alloc.update([].into_iter(), [(5, 3), (6, 3)]);
  assert_eq!(r.resize_to, Some(38));
  assert!(r.failed_to_allocate.is_empty());
  for k in 1..=6 {
    let (size, offset) = alloc.get_region(&k).unwrap();
    assert_eq!(size, 3);
    assert_eq!(offset % 4, 0);
  }
  assert_exclusive(&r);
}

#[test]
fn alignment_reach_max_reports_failure() {
  let mut alloc = new_aligned_alloc(16, 16);
  alloc.update([].into_iter(), [(1, 3), (2, 3), (3, 3), (4, 3)]);
  let r = alloc.update([].into_iter(), [(5, 3)]);
  assert!(r.failed_to_allocate.contains(&5));
  assert_eq!(alloc.used_count, 12);
  assert_eq!(alloc.get_region(&5), None);
  assert_exclusive(&r);
}

#[test]
fn alignment_used_count_still_matches_ranges() {
  let mut alloc = new_aligned_alloc(16, 64);
  alloc.update([].into_iter(), [(1, 3), (2, 5), (3, 1)]);
  alloc.update([2].into_iter(), [(2, 2), (4, 7)]);
  let r = alloc.update([1, 4].into_iter(), [(1, 3), (5, 9)]);
  let expected: u32 = alloc.ranges.values().map(|&(s, _, _)| s).sum();
  assert_eq!(alloc.used_count, expected);
  assert!(alloc.current_count >= alloc.used_count);
  for (k, &(size, offset, _)) in &alloc.ranges {
    assert_eq!(alloc.get_region(k), Some((size, offset)));
    assert_eq!(offset % 4, 0);
  }
  assert_exclusive(&r);
}

#[test]
fn shrink_releases_capacity_when_under_utilized() {
  let mut alloc = new_alloc(8, 64);
  alloc.update([].into_iter(), (1..=20).map(|k| (k, 1)));
  let r = alloc.update((2..=17).into_iter(), []);
  assert_eq!(r.resize_to, Some(8));
  assert_eq!(r.data_movements.len(), 4);
  assert_eq!(alloc.current_count, 8);
  let mut offsets = Vec::new();
  for k in [1, 18, 19, 20] {
    let (size, offset) = alloc.get_region(&k).unwrap();
    assert_eq!(size, 1);
    assert!(offset < 8);
    offsets.push(offset);
  }
  offsets.sort();
  assert_eq!(offsets, [0, 1, 2, 3]);
  assert_exclusive(&r);
}

#[test]
fn shrink_disabled_by_config() {
  let mut alloc = new_alloc(8, 64).with_shrink(false);
  alloc.update([].into_iter(), (1..=20).map(|k| (k, 1)));
  let r = alloc.update((2..=17).into_iter(), []);
  assert_eq!(r.resize_to, None);
  assert!(r.data_movements.is_empty());
  assert_eq!(alloc.current_count, 22);
}

#[test]
fn shrink_stops_at_init_count() {
  let mut alloc = new_alloc(16, 64);
  alloc.update([].into_iter(), (1..=32).map(|k| (k, 1)));
  let r = alloc.update((2..=31).into_iter(), []);
  assert_eq!(r.resize_to, Some(16));
  let r = alloc.update([2].into_iter(), []);
  assert_eq!(r.resize_to, None);
  assert_eq!(alloc.current_count, 16);
}

#[test]
fn shrink_not_triggered_when_capacity_used() {
  let mut alloc = new_alloc(8, 64);
  alloc.update([].into_iter(), (1..=8).map(|k| (k, 1)));
  let r = alloc.update((2..=5).into_iter(), []);
  assert_eq!(r.resize_to, None);
  assert_eq!(alloc.current_count, 8);
}

#[test]
fn shrink_with_new_allocations_same_update() {
  let mut alloc = new_alloc(4, 64);
  alloc.update([].into_iter(), [(1, 1), (2, 1), (3, 1), (4, 1)]);
  alloc.update([].into_iter(), [(5, 1), (6, 1), (7, 1), (8, 1)]);
  let r = alloc.update((2..=7).into_iter(), [(9, 1)]);
  assert_eq!(r.resize_to, Some(6));
  assert_eq!(alloc.current_count, 6);
  for k in [1, 8, 9] {
    let (size, offset) = alloc.get_region(&k).unwrap();
    assert_eq!(size, 1);
    assert!(offset < 6);
  }
  assert_eq!(
    r.new_data_to_write.get(&9).unwrap().0,
    alloc.get_region(&9).unwrap().1
  );
  assert_exclusive(&r);
}

#[test]
fn shrink_then_grow_again() {
  let mut alloc = new_alloc(4, 64);
  alloc.update([].into_iter(), (1..=8).map(|k| (k, 1)));
  alloc.update((2..=8).into_iter(), []);
  assert_eq!(alloc.current_count, 4);
  alloc.update([].into_iter(), (9..=12).map(|k| (k, 1)));
  assert_eq!(alloc.current_count, 5);
  for k in [1, 9, 10, 11, 12] {
    assert_eq!(alloc.get_region(&k).unwrap().0, 1);
  }
  assert_eq!(alloc.used_count, 5);
}

#[test]
fn alignment_shrink_is_exempt() {
  let mut alloc = new_aligned_alloc(8, 64);
  alloc.update([].into_iter(), [(1, 3), (2, 3), (3, 3), (4, 3)]);
  let r = alloc.update((2..=4).into_iter(), []);
  assert_eq!(r.resize_to, None);
  // 13 slots only fit three 3-byte items aligned to 4, the fourth triggers grow to 26
  assert_eq!(alloc.current_count, 26);
}

#[test]
fn result_iterators_match_final_state() {
  let mut alloc = new_alloc(4, 64);
  alloc.update([].into_iter(), [(1, 2)]);
  let r = alloc.update([].into_iter(), [(2, 2)]);
  for (k, change) in r.iter_update_or_insert() {
    match change {
      AllocateChangeType::Allocated([offset, count]) => {
        assert_eq!(alloc.get_region(&k), Some((count, offset)));
      }
      AllocateChangeType::FailedToAllocate => panic!("should not fail"),
    }
  }
  assert_exclusive(&r);
}

#[test]
fn access_new_change_reports_failed() {
  let mut alloc = new_alloc(4, 4);
  alloc.update([].into_iter(), [(1, 2), (2, 2)]);
  let r = alloc.update([].into_iter(), [(3, 2)]);
  assert!(matches!(
    r.access_new_change(3),
    Some(AllocateChangeType::FailedToAllocate)
  ));
  assert!(r.access_new_change(1).is_none());
  assert!(r.access_new_change(9).is_none());
}
