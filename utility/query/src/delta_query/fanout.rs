use crate::*;

/// Incremental fan-out: propagates changes through a 1:N relationship.
///
/// The upstream maps AKey to XValue. The relation maps each BKey to one AKey and the
/// rev relation is its inverse, mapping AKey to the set of BKey. The fan-out result is
/// the chain relation -> upstream, keyed by BKey, and this type is the delta of it.
///
/// The change of a BKey is fully determined by its previous chained value and its
/// current chained value, so `access` is a pure function of the key. A BKey can only
/// have changed if its relation changed, or if the AKey it currently relates to has an
/// upstream change. `iter_key_value` enumerates exactly these candidates and reuses
/// `access` for the value, which keeps the two consistent by construction.
#[derive(Clone)]
pub struct FanoutValueChange<Up, UpD, Rev, Rel, RelD> {
  pub upstream: Up,
  pub upstream_delta: UpD,
  pub rev_relation: Rev,
  pub relation: Rel,
  pub relation_delta: RelD,
}

#[allow(type_alias_bounds)]
pub type FanoutDualQuery<S: DualQueryLike, R: TriQueryLike> = DualQuery<
  ChainQuery<R::View, S::View>,
  FanoutValueChange<S::View, S::Delta, R::InvView, R::View, R::Delta>,
>;

impl<A, B, X, Up, UpD, Rev, Rel, RelD> Query for FanoutValueChange<Up, UpD, Rev, Rel, RelD>
where
  A: CKey,
  B: CKey,
  X: CValue,
  Up: Query<Key = A, Value = X>,
  UpD: Query<Key = A, Value = ValueChange<X>>,
  Rev: MultiQuery<Key = A, Value = B>,
  Rel: Query<Key = B, Value = A>,
  RelD: Query<Key = B, Value = ValueChange<A>>,
{
  type Key = B;
  type Value = ValueChange<X>;

  fn iter_key_value(&self) -> impl Iterator<Item = (B, ValueChange<X>)> + '_ {
    let relation_changed = self.relation_delta.iter_key_value().map(|(b, _)| b);

    // the iterator returned by access_multi borrows its key, so the affected b keys
    // are collected first. each b relates to exactly one a, so the only possible
    // duplication is with the relation changed part, which is filtered out here
    let mut upstream_changed = Vec::new();
    for (a, _) in self.upstream_delta.iter_key_value() {
      self
        .rev_relation
        .access_multi_visitor(&a, &mut |b| upstream_changed.push(b));
    }
    let upstream_changed = upstream_changed
      .into_iter()
      .filter(|b| !self.relation_delta.contains(b));

    let iter = relation_changed
      .chain(upstream_changed)
      .filter_map(|b| self.access(&b).map(|change| (b, change)));

    avoid_huge_debug_symbols_by_boxing_iter(iter)
  }

  fn access(&self, b: &B) -> Option<ValueChange<X>> {
    let (previous_a, current_a) = match self.relation_delta.access(b) {
      Some(ValueChange::Delta(a, previous_a)) => (previous_a, Some(a)),
      Some(ValueChange::Remove(previous_a)) => (Some(previous_a), None),
      None => {
        let a = self.relation.access(b)?;
        if !self.upstream_delta.contains(&a) {
          return None;
        }
        (Some(a.clone()), Some(a))
      }
    };

    let previous_upstream = make_previous(&self.upstream, &self.upstream_delta);
    let previous_x = previous_a.and_then(|a| previous_upstream.access(&a));
    let current_x = current_a.and_then(|a| self.upstream.access(&a));

    match (previous_x, current_x) {
      (previous_x, Some(x)) => Some(ValueChange::Delta(x, previous_x)),
      (Some(previous_x), None) => Some(ValueChange::Remove(previous_x)),
      (None, None) => None,
    }
  }

  fn has_item_hint(&self) -> bool {
    self.relation_delta.has_item_hint() || self.upstream_delta.has_item_hint()
  }
}

// tests

// Helper types for readable tests:
// AKey = u32  (the "one" side, upstream key)
// BKey = u32  (the "many" side, downstream key)
// XValue = i32 (the payload value)
//
// every test describes a consistent state: upstream and relation are the current
// views, the deltas describe how they reached this state, and rev_relation is the
// inverse of relation.

#[cfg(test)]
fn fanout_changes(
  upstream: FastHashMap<u32, i32>,
  upstream_delta: FastHashMap<u32, ValueChange<i32>>,
  rev_relation: FastHashMap<u32, FastHashSet<u32>>,
  relation: FastHashMap<u32, u32>,
  relation_delta: FastHashMap<u32, ValueChange<u32>>,
) -> FastHashMap<u32, ValueChange<i32>> {
  let changes = FanoutValueChange {
    upstream,
    upstream_delta,
    rev_relation,
    relation,
    relation_delta,
  };

  validate_query_consistency(&changes);

  let pairs: Vec<_> = changes.iter_key_value().collect();
  let collected: FastHashMap<_, _> = pairs.iter().cloned().collect();
  assert_eq!(
    pairs.len(),
    collected.len(),
    "iter_key_value should not yield duplicate keys"
  );
  collected
}

// === relation changes ===

#[test]
fn test_fanout_relational_insert() {
  // B-key 100 now relates to A-key 1 (was unrelated); upstream(1) = 10
  let delta = fanout_changes(
    FastHashMap::from_iter([(1, 10)]),
    FastHashMap::default(),
    FastHashMap::from_iter([(1, FastHashSet::from_iter([100]))]),
    FastHashMap::from_iter([(100, 1)]),
    FastHashMap::from_iter([(100, ValueChange::Delta(1, None))]),
  );

  assert_eq!(delta.len(), 1);
  assert_eq!(delta[&100], ValueChange::Delta(10, None));
}

#[test]
fn test_fanout_relational_update() {
  // B-key 200: relation changed from A=1 to A=2; upstream(1)=10, upstream(2)=20
  let delta = fanout_changes(
    FastHashMap::from_iter([(1, 10), (2, 20)]),
    FastHashMap::default(),
    FastHashMap::from_iter([(2, FastHashSet::from_iter([200]))]),
    FastHashMap::from_iter([(200, 2)]),
    FastHashMap::from_iter([(200, ValueChange::Delta(2, Some(1)))]),
  );

  assert_eq!(delta.len(), 1);
  assert_eq!(delta[&200], ValueChange::Delta(20, Some(10)));
}

#[test]
fn test_fanout_relational_update_new_a_missing() {
  // B-key 100: relation changed from A=1 to A=2; upstream(1)=10, upstream(2) missing
  let delta = fanout_changes(
    FastHashMap::from_iter([(1, 10)]),
    FastHashMap::default(),
    FastHashMap::from_iter([(2, FastHashSet::from_iter([100]))]),
    FastHashMap::from_iter([(100, 2)]),
    FastHashMap::from_iter([(100, ValueChange::Delta(2, Some(1)))]),
  );

  assert_eq!(delta.len(), 1);
  assert_eq!(delta[&100], ValueChange::Remove(10));
}

#[test]
fn test_fanout_relational_update_both_a_missing() {
  // B-key 100: relation changed from A=1 to A=2; neither has an upstream value
  let delta = fanout_changes(
    FastHashMap::default(),
    FastHashMap::default(),
    FastHashMap::from_iter([(2, FastHashSet::from_iter([100]))]),
    FastHashMap::from_iter([(100, 2)]),
    FastHashMap::from_iter([(100, ValueChange::Delta(2, Some(1)))]),
  );

  assert!(delta.is_empty());
}

#[test]
fn test_fanout_relational_remove() {
  // B-key 300: relation removed (was A=1); upstream(1)=10
  let delta = fanout_changes(
    FastHashMap::from_iter([(1, 10)]),
    FastHashMap::default(),
    FastHashMap::default(),
    FastHashMap::default(),
    FastHashMap::from_iter([(300, ValueChange::Remove(1))]),
  );

  assert_eq!(delta.len(), 1);
  assert_eq!(delta[&300], ValueChange::Remove(10));
}

#[test]
fn test_fanout_relational_remove_missing_upstream() {
  // B-key 300: relation removed (was A=1); upstream(1) does not exist
  let delta = fanout_changes(
    FastHashMap::default(),
    FastHashMap::default(),
    FastHashMap::default(),
    FastHashMap::default(),
    FastHashMap::from_iter([(300, ValueChange::Remove(1))]),
  );

  assert!(delta.is_empty());
}

#[test]
fn test_fanout_relational_multiple() {
  // B-key 100: new → A=1 (upstream(1)=10)
  // B-key 200: A=1→A=2 (upstream(1)=10, upstream(2)=20)
  // B-key 300: remove A=1 (upstream(1)=10)
  let delta = fanout_changes(
    FastHashMap::from_iter([(1, 10), (2, 20)]),
    FastHashMap::default(),
    FastHashMap::from_iter([
      (1, FastHashSet::from_iter([100])),
      (2, FastHashSet::from_iter([200])),
    ]),
    FastHashMap::from_iter([(100, 1), (200, 2)]),
    FastHashMap::from_iter([
      (100, ValueChange::Delta(1, None)),
      (200, ValueChange::Delta(2, Some(1))),
      (300, ValueChange::Remove(1)),
    ]),
  );

  assert_eq!(delta.len(), 3);
  assert_eq!(delta[&100], ValueChange::Delta(10, None));
  assert_eq!(delta[&200], ValueChange::Delta(20, Some(10)));
  assert_eq!(delta[&300], ValueChange::Remove(10));
}

// === upstream changes ===

#[test]
fn test_fanout_upstream_delta() {
  // A-key 1's value changed from 10 to 15, B-keys 100, 101 relate to A-key 1
  // B-key 200 relates to the unchanged A-key 2 and must not appear
  let delta = fanout_changes(
    FastHashMap::from_iter([(1, 15), (2, 20)]),
    FastHashMap::from_iter([(1, ValueChange::Delta(15, Some(10)))]),
    FastHashMap::from_iter([
      (1, FastHashSet::from_iter([100, 101])),
      (2, FastHashSet::from_iter([200])),
    ]),
    FastHashMap::from_iter([(100, 1), (101, 1), (200, 2)]),
    FastHashMap::default(),
  );

  assert_eq!(delta.len(), 2);
  assert_eq!(delta[&100], ValueChange::Delta(15, Some(10)));
  assert_eq!(delta[&101], ValueChange::Delta(15, Some(10)));
}

#[test]
fn test_fanout_upstream_insert() {
  // B-key 100 already relates to A-key 1, and A-key 1 now gets its first value
  let delta = fanout_changes(
    FastHashMap::from_iter([(1, 15)]),
    FastHashMap::from_iter([(1, ValueChange::Delta(15, None))]),
    FastHashMap::from_iter([(1, FastHashSet::from_iter([100]))]),
    FastHashMap::from_iter([(100, 1)]),
    FastHashMap::default(),
  );

  assert_eq!(delta.len(), 1);
  assert_eq!(delta[&100], ValueChange::Delta(15, None));
}

#[test]
fn test_fanout_upstream_remove() {
  // A-key 1's value was removed, B-keys 100, 101 relate to A-key 1
  let delta = fanout_changes(
    FastHashMap::default(),
    FastHashMap::from_iter([(1, ValueChange::Remove(10))]),
    FastHashMap::from_iter([(1, FastHashSet::from_iter([100, 101]))]),
    FastHashMap::from_iter([(100, 1), (101, 1)]),
    FastHashMap::default(),
  );

  assert_eq!(delta.len(), 2);
  assert_eq!(delta[&100], ValueChange::Remove(10));
  assert_eq!(delta[&101], ValueChange::Remove(10));
}

// === relation and upstream change at the same time ===

#[test]
fn test_fanout_overlap_relation_update_and_upstream_insert() {
  // B-key 100: relation changed from A=1 to A=2, and A=2 got its first value 20
  // B-key 100 is reachable from both the relation delta and the rev relation of A=2,
  // it must be reported once, comparing previous chained value 10 with current 20
  let delta = fanout_changes(
    FastHashMap::from_iter([(1, 10), (2, 20)]),
    FastHashMap::from_iter([(2, ValueChange::Delta(20, None))]),
    FastHashMap::from_iter([(2, FastHashSet::from_iter([100]))]),
    FastHashMap::from_iter([(100, 2)]),
    FastHashMap::from_iter([(100, ValueChange::Delta(2, Some(1)))]),
  );

  assert_eq!(delta.len(), 1);
  assert_eq!(delta[&100], ValueChange::Delta(20, Some(10)));
}

#[test]
fn test_fanout_overlap_relation_update_and_upstream_remove() {
  // B-key 100: relation changed from A=1 to A=2, and A=2's value 20 was removed
  let delta = fanout_changes(
    FastHashMap::from_iter([(1, 10)]),
    FastHashMap::from_iter([(2, ValueChange::Remove(20))]),
    FastHashMap::from_iter([(2, FastHashSet::from_iter([100]))]),
    FastHashMap::from_iter([(100, 2)]),
    FastHashMap::from_iter([(100, ValueChange::Delta(2, Some(1)))]),
  );

  assert_eq!(delta.len(), 1);
  assert_eq!(delta[&100], ValueChange::Remove(10));
}

#[test]
fn test_fanout_overlap_relation_remove_and_upstream_change() {
  // B-key 100: relation to A=1 removed while A=1's value changed from 10 to 15
  // B-key 101 keeps relating to A=1 and sees the value change
  let delta = fanout_changes(
    FastHashMap::from_iter([(1, 15)]),
    FastHashMap::from_iter([(1, ValueChange::Delta(15, Some(10)))]),
    FastHashMap::from_iter([(1, FastHashSet::from_iter([101]))]),
    FastHashMap::from_iter([(101, 1)]),
    FastHashMap::from_iter([(100, ValueChange::Remove(1))]),
  );

  assert_eq!(delta.len(), 2);
  assert_eq!(delta[&100], ValueChange::Remove(10));
  assert_eq!(delta[&101], ValueChange::Delta(15, Some(10)));
}

#[test]
fn test_fanout_dual_query() {
  // exercise the DualQueryLike::fanout entry, view is relation chain upstream
  let upstream = DualQuery {
    view: FastHashMap::from_iter([(1, 15), (2, 20)]),
    delta: FastHashMap::from_iter([(1, ValueChange::Delta(15, Some(10)))]),
  };
  let relation = TriQuery {
    base: DualQuery {
      view: FastHashMap::from_iter([(100, 1), (200, 2)]),
      delta: FastHashMap::from_iter([(200, ValueChange::Delta(2, None))]),
    },
    rev_many_view: FastHashMap::from_iter([
      (1, FastHashSet::from_iter([100])),
      (2, FastHashSet::from_iter([200])),
    ]),
  };

  let result = upstream.fanout(relation);

  assert_eq!(result.view.access(&100), Some(15));
  assert_eq!(result.view.access(&200), Some(20));
  assert_eq!(result.view.access(&300), None);

  validate_query_consistency(&result.delta);
  assert_eq!(
    result.delta.access(&100),
    Some(ValueChange::Delta(15, Some(10)))
  );
  assert_eq!(
    result.delta.access(&200),
    Some(ValueChange::Delta(20, None))
  );
  assert_eq!(result.delta.access(&300), None);
}
