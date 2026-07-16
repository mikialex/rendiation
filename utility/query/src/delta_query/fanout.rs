use crate::*;

/// Incremental fan-out: propagates changes through a 1:N relationship.
///
/// Given:
/// - `getter`:         AKey → XValue  (the value of each A-key)
/// - `upstream_changes`:  (AKey, ValueChange<XValue>)  (how A-key values changed)
/// - `rev_many_view`:  MultiQuery<AKey, BKey>  (reverse: which B-keys relate to which A-key)
/// - `relation_access`:   BKey → AKey  (the current relation mapping)
/// - `relational_changes`: (BKey, ValueChange<AKey>)  (how relations changed)
///
/// Produces a DualQuery from BKey → XValue, where the delta captures net
/// changes induced by both relation mutations and upstream value mutations.
///
/// The algorithm runs in two phases:
/// 1. Process relational changes — for each changed relation, look up the
///    getter to produce output Delta/Remove entries.
/// 2. Process upstream value changes — for each changed A-key, fan out
///    through the reverse relation to affected B-keys, producing output
///    entries.  Entries that cancel with Phase 1 results are removed.
#[inline(always)]
pub fn fanout_impl<AKey, BKey, XValue, Getter, GetterDelta, RevMany, RelAccess, RelDelta>(
  getter: Getter,
  upstream_changes: GetterDelta,
  rev_many_view: RevMany,
  relation_access: RelAccess,
  relational_changes: RelDelta,
) -> DualQuery<ChainQuery<RelAccess, Getter>, Arc<FastHashMap<BKey, ValueChange<XValue>>>>
where
  AKey: CKey,
  BKey: CKey,
  XValue: CValue,
  Getter: Query<Key = AKey, Value = XValue> + Clone + 'static,
  GetterDelta: Query<Key = AKey, Value = ValueChange<XValue>> + Clone + 'static,
  RevMany: MultiQuery<Key = AKey, Value = BKey> + Clone + 'static,
  RelAccess: Query<Key = BKey, Value = AKey> + Clone + 'static,
  RelDelta: Query<Key = BKey, Value = ValueChange<AKey>> + Clone + 'static,
{
  let getter_previous = make_previous(&getter, &upstream_changes);
  let one_acc_previous = make_previous(&relation_access, &relational_changes);

  let relational_changes_iter = relational_changes.iter_key_value();
  let upstream_changes_iter = upstream_changes.iter_key_value();

  let output_reserve = relational_changes_iter.size_hint().0 + upstream_changes_iter.size_hint().0;

  let mut output = FastHashMap::with_capacity_and_hasher(output_reserve, Default::default());

  // Phase 1: relational changes
  {
    relational_changes_iter.for_each(|(b_key, change)| match change {
      ValueChange::Delta(new_a, old_a) => {
        let prev_x = old_a.and_then(|old_a| getter_previous.access(&old_a));
        if let Some(new_x) = getter.access(&new_a) {
          output.insert(b_key.clone(), ValueChange::Delta(new_x, prev_x));
        } else if let Some(prev_x) = prev_x {
          output.insert(b_key.clone(), ValueChange::Remove(prev_x));
        }
      }
      ValueChange::Remove(old_a) => {
        if let Some(prev_x) = getter_previous.access(&old_a) {
          output.insert(b_key.clone(), ValueChange::Remove(prev_x));
        }
      }
    });
  }

  // Phase 2: upstream value changes
  {
    for (a_key, delta) in upstream_changes_iter {
      match delta {
        ValueChange::Remove(_p) => rev_many_view.access_multi_visitor(&a_key, &mut |b_key| {
          if let Some(prev_a) = one_acc_previous.access(&b_key) {
            if let Some(prev_x) = getter_previous.access(&prev_a) {
              if let Some(ValueChange::Delta(_, _)) = output.get(&b_key) {
                output.remove(&b_key);
              } else {
                output.insert(b_key.clone(), ValueChange::Remove(prev_x));
              }
            }
          }
        }),
        ValueChange::Delta(new_x, _p) => rev_many_view.access_multi_visitor(&a_key, &mut |b_key| {
          if let Some(prev_a) = one_acc_previous.access(&b_key) {
            let prev_x = getter_previous.access(&prev_a);
            if let Some(ValueChange::Remove(_)) = output.get(&b_key) {
              output.remove(&b_key);
            } else {
              output.insert(b_key.clone(), ValueChange::Delta(new_x.clone(), prev_x));
            }
          } else {
            #[allow(clippy::collapsible_else_if)]
            if let Some(ValueChange::Remove(_)) = output.get(&b_key) {
              output.remove(&b_key);
            } else {
              output.insert(b_key.clone(), ValueChange::Delta(new_x.clone(), None));
            }
          }
        }),
      }
    }
  }

  let d = Arc::new(output);
  let v = relation_access.chain(getter);

  DualQuery { view: v, delta: d }
}

// tests

// Helper types for readable tests:
// AKey = u32  (the "one" side, upstream key)
// BKey = u32  (the "many" side, downstream key)
// XValue = i32 (the payload value)

// === Phase 1: relational changes ===

#[test]
fn test_fanout_relational_insert() {
  // B-key 100 now relates to A-key 1 (was unrelated); getter(1) = 10
  let getter = FastHashMap::from_iter([(1u32, 10i32)]);
  let mut rel_changes = FastHashMap::default();
  rel_changes.insert(100u32, ValueChange::Delta(1u32, None));

  let result = fanout_impl(
    getter,
    FastHashMap::default(),
    FastHashMap::default(),
    FastHashMap::default(), // relation_access (not needed for Phase 1)
    rel_changes,
  );

  let delta = result.delta;
  assert_eq!(delta.len(), 1);
  assert_eq!(delta[&100], ValueChange::Delta(10, None));
}

#[test]
fn test_fanout_relational_update() {
  // B-key 200: relation changed from A=1 to A=2; getter(1)=10, getter(2)=20
  let getter = FastHashMap::from_iter([(1u32, 10i32), (2, 20)]);
  let mut rel_changes = FastHashMap::default();
  rel_changes.insert(200u32, ValueChange::Delta(2u32, Some(1u32)));

  let result = fanout_impl(
    getter,
    FastHashMap::default(),
    FastHashMap::default(),
    FastHashMap::default(),
    rel_changes,
  );

  let delta = result.delta;
  assert_eq!(delta.len(), 1);
  assert_eq!(delta[&200], ValueChange::Delta(20, Some(10)));
}

#[test]
fn test_fanout_relational_update_new_a_missing() {
  // B-key 100: relation changed from A=1 to A=2; getter(1)=10, getter(2) missing
  let getter = FastHashMap::from_iter([(1u32, 10i32)]);
  let mut rel_changes = FastHashMap::default();
  rel_changes.insert(100u32, ValueChange::Delta(2u32, Some(1u32)));

  let result = fanout_impl(
    getter,
    FastHashMap::default(),
    FastHashMap::default(),
    FastHashMap::default(),
    rel_changes,
  );

  let delta = result.delta;
  assert_eq!(delta.len(), 1);
  // new A has no getter value, old A has getter value 10 → Remove(10)
  assert_eq!(delta[&100], ValueChange::Remove(10));
}

#[test]
fn test_fanout_relational_remove() {
  // B-key 300: relation removed (was A=1); getter(1)=10
  let getter = FastHashMap::from_iter([(1u32, 10i32)]);
  let mut rel_changes = FastHashMap::default();
  rel_changes.insert(300u32, ValueChange::Remove(1u32));

  let result = fanout_impl(
    getter,
    FastHashMap::default(),
    FastHashMap::default(),
    FastHashMap::default(),
    rel_changes,
  );

  let delta = result.delta;
  assert_eq!(delta.len(), 1);
  assert_eq!(delta[&300], ValueChange::Remove(10));
}

#[test]
fn test_fanout_relational_remove_missing_getter() {
  // B-key 300: relation removed (was A=1); getter(1) no longer exists
  let getter: FastHashMap<u32, i32> = FastHashMap::default();
  let mut rel_changes = FastHashMap::default();
  rel_changes.insert(300u32, ValueChange::Remove(1u32));

  let result = fanout_impl(
    getter,
    FastHashMap::default(),
    FastHashMap::default(),
    FastHashMap::default(),
    rel_changes,
  );

  let delta = result.delta;
  // getter_previous has no entry for A=1 → no output
  assert!(delta.is_empty());
}

#[test]
fn test_fanout_relational_multiple() {
  // B-key 100: new → A=1 (getter(1)=10)
  // B-key 200: A=1→A=2 (getter(1)=10, getter(2)=20)
  // B-key 300: remove A=1 (getter(1)=10)
  let getter = FastHashMap::from_iter([(1u32, 10i32), (2, 20)]);
  let mut rel_changes = FastHashMap::default();
  rel_changes.insert(100u32, ValueChange::Delta(1u32, None));
  rel_changes.insert(200u32, ValueChange::Delta(2u32, Some(1u32)));
  rel_changes.insert(300u32, ValueChange::Remove(1u32));

  let result = fanout_impl(
    getter,
    FastHashMap::default(),
    FastHashMap::default(),
    FastHashMap::default(),
    rel_changes,
  );

  let delta = result.delta;
  assert_eq!(delta.len(), 3);
  assert_eq!(delta[&100], ValueChange::Delta(10, None));
  assert_eq!(delta[&200], ValueChange::Delta(20, Some(10)));
  assert_eq!(delta[&300], ValueChange::Remove(10));
}

// === Phase 2: upstream value changes ===

#[test]
fn test_fanout_upstream_delta() {
  // A-key 1's value changed from 10 to 15
  // B-keys 100, 101 both relate to A-key 1
  let getter = FastHashMap::from_iter([(1u32, 15i32)]);
  let mut up_changes = FastHashMap::default();
  up_changes.insert(1u32, ValueChange::Delta(15i32, Some(10)));

  let mut rev_many = FastHashMap::default();
  rev_many.insert(1u32, FastHashSet::from_iter([100u32, 101]));

  let rel_access = FastHashMap::from_iter([(100u32, 1u32), (101, 1)]);

  let result = fanout_impl(
    getter,
    up_changes,
    rev_many,
    rel_access,
    FastHashMap::default(),
  );

  let delta = result.delta;
  assert_eq!(delta.len(), 2);
  assert_eq!(delta[&100], ValueChange::Delta(15, Some(10)));
  assert_eq!(delta[&101], ValueChange::Delta(15, Some(10)));
}

#[test]
fn test_fanout_upstream_delta_new_relation() {
  // A-key 1's value changed, but B-key 100 has no previous relation record
  let getter = FastHashMap::from_iter([(1u32, 15i32)]);
  let mut up_changes = FastHashMap::default();
  up_changes.insert(1u32, ValueChange::Delta(15i32, None));

  let mut rev_many = FastHashMap::default();
  rev_many.insert(1u32, FastHashSet::from_iter([100u32]));

  // rel_access has no entry for 100 (new relation)
  let rel_access: FastHashMap<u32, u32> = FastHashMap::default();

  let result = fanout_impl(
    getter,
    up_changes,
    rev_many,
    rel_access,
    FastHashMap::default(),
  );

  let delta = result.delta;
  assert_eq!(delta.len(), 1);
  assert_eq!(delta[&100], ValueChange::Delta(15, None));
}

#[test]
fn test_fanout_upstream_remove() {
  // A-key 1's value was removed (X gone)
  // B-keys 100, 101 relate to A-key 1
  let getter: FastHashMap<u32, i32> = FastHashMap::default();
  let mut up_changes = FastHashMap::default();
  up_changes.insert(1u32, ValueChange::Remove(10i32));

  let mut rev_many = FastHashMap::default();
  rev_many.insert(1u32, FastHashSet::from_iter([100u32, 101]));

  let rel_access = FastHashMap::from_iter([(100u32, 1u32), (101, 1)]);

  let result = fanout_impl(
    getter,
    up_changes,
    rev_many,
    rel_access,
    FastHashMap::default(),
  );

  let delta = result.delta;
  assert_eq!(delta.len(), 2);
  assert_eq!(delta[&100], ValueChange::Remove(10));
  assert_eq!(delta[&101], ValueChange::Remove(10));
}

// === Cancel: Phase 1 + Phase 2 interaction ===

#[test]
fn test_fanout_cancel_delta_remove() {
  // Phase 1: relational change inserts Delta at B-key 100
  // Phase 2: upstream Remove at the same B-key 100 → cancels out
  let getter = FastHashMap::from_iter([(1u32, 10i32), (2, 20)]);

  // Relational change: B-key 100 changes from A=1 to A=2
  // Phase 1 → Delta(20, Some(10))
  let mut rel_changes = FastHashMap::default();
  rel_changes.insert(100u32, ValueChange::Delta(2u32, Some(1u32)));

  // Upstream: A-key 2's value is removed
  // Phase 2: for B-key 100 (which relates to A=2 via prev relation), emit Remove(20)
  // But output already has Delta(20, _) → cancel!
  let mut up_changes = FastHashMap::default();
  up_changes.insert(2u32, ValueChange::Remove(20i32));

  let mut rev_many = FastHashMap::default();
  rev_many.insert(2u32, FastHashSet::from_iter([100u32]));

  // relation_access: B-key 100 currently relates to A=2
  let rel_access = FastHashMap::from_iter([(100u32, 2u32)]);

  let result = fanout_impl(getter, up_changes, rev_many, rel_access, rel_changes);

  let delta = result.delta;
  // Phase 1 inserted Delta(20, Some(10)) for key 100
  // Phase 2 tried to insert Remove(20) but found Delta → removed
  assert!(delta.is_empty());
}

#[test]
fn test_fanout_cancel_remove_delta() {
  // Phase 1: relational change inserts Remove at B-key 100
  // Phase 2: upstream Delta at the same B-key 100 → cancels out
  let getter = FastHashMap::from_iter([(1u32, 10i32), (2, 20)]);

  // Relational change: B-key 100 was related to A=1, now removed
  // Phase 1 → Remove(10)
  let mut rel_changes = FastHashMap::default();
  rel_changes.insert(100u32, ValueChange::Remove(1u32));

  // Upstream: A-key 2's value changed
  // Phase 2: B-key 100 is related to A=2 (new relation, no previous A)
  // → Delta(20, None)
  let mut up_changes = FastHashMap::default();
  up_changes.insert(2u32, ValueChange::Delta(20i32, None));

  let mut rev_many = FastHashMap::default();
  rev_many.insert(2u32, FastHashSet::from_iter([100u32]));

  // rel_access has no history for B=100; it's a new relation through A=2
  let rel_access: FastHashMap<u32, u32> = FastHashMap::default();

  let result = fanout_impl(getter, up_changes, rev_many, rel_access, rel_changes);

  let delta = result.delta;
  // Phase 1 inserted Remove(10) for key 100
  // Phase 2 tried Delta(20, None) for key 100; found Remove → removed
  assert!(delta.is_empty());
}

#[test]
fn test_fanout_cancel_partial() {
  // Phase 1: Delta at B-key 100, Delta at B-key 101
  // Phase 2: upstream Remove cancels only B-key 100
  let getter = FastHashMap::from_iter([(1u32, 10i32), (2, 20)]);

  // Relational: 100 changes A=1→A=2, 101 changes A=1→A=2
  let mut rel_changes = FastHashMap::default();
  rel_changes.insert(100u32, ValueChange::Delta(2u32, Some(1u32)));
  rel_changes.insert(101u32, ValueChange::Delta(2u32, Some(1u32)));

  // Upstream: A=2 removed
  let mut up_changes = FastHashMap::default();
  up_changes.insert(2u32, ValueChange::Remove(20i32));

  // B-keys 100 and 101 both relate to A=2
  let mut rev_many = FastHashMap::default();
  rev_many.insert(1u32, FastHashSet::from_iter([100u32, 101]));
  rev_many.insert(2u32, FastHashSet::from_iter([100u32, 101]));

  let rel_access = FastHashMap::from_iter([(100u32, 2u32), (101, 2)]);

  let result = fanout_impl(getter, up_changes, rev_many, rel_access, rel_changes);

  let delta = result.delta;
  // Both B-keys 100 and 101 should be cancelled
  assert!(delta.is_empty());
}

#[test]
fn test_fanout_view_returns_composed_query() {
  // Verify the view is correctly composed (rel_access chain getter)
  let getter = FastHashMap::from_iter([(1u32, 10i32), (2, 20)]);
  let rel_access = FastHashMap::from_iter([(100u32, 1u32), (200, 2)]);

  let result = fanout_impl(
    getter,
    FastHashMap::default(),
    FastHashMap::default(),
    rel_access,
    FastHashMap::default(),
  );

  let view = result.view;
  // view = rel_access.chain(getter): B-key 100 → A-key 1 → value 10
  assert_eq!(view.access(&100), Some(10));
  assert_eq!(view.access(&200), Some(20));
  assert_eq!(view.access(&300), None);
}
