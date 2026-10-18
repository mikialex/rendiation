use crate::*;

/// Incrementally maintain `derive[node] = derive_logic(payload[node], derive[parent])` for a forest.
///
/// The change set is the union of alive nodes in the payload change and the connectivity change.
/// For each changed node we walk up its ancestors to find out if it has a changed ancestor. The
/// walked path is memoized, so every node is walked at most once, and the cost is the size of the
/// union of the paths. A changed node without changed ancestor is an update root. A changed node
/// with changed ancestor records the edges from its nearest changed ancestor down to itself as
/// dirty edges, the traversal of the ancestor must follow them even if the ancestor's derive
/// result is not changed.
///
/// Then we traverse from each update root in parent first order. A node is recomputed only if it
/// is changed or its parent's derive result is changed. If the derive result is changed, all the
/// children are visited, otherwise only the dirty edges are followed.
///
/// The connectivity must not contain cycles, this is checked and panics.
pub fn compute_tree_derive<K: CKey, T: CValue>(
  derive: &mut dyn QueryLikeMutateTarget<K, T>,
  derive_logic: impl Fn(&T, Option<&T>) -> T + Send + Sync + 'static + Copy,
  payload_source: impl Query<Key = K, Value = T>,
  payload_change: impl Query<Key = K, Value = ValueChange<T>>,
  connectivity_source: impl Query<Key = K, Value = K>,
  connectivity_source_rev: impl MultiQuery<Key = K, Value = K>,
  connectivity_change: impl Query<Key = K, Value = ValueChange<K>>,
) {
  let mut changed = FastHashSet::default();
  for (k, _) in connectivity_change.iter_key_value() {
    changed.insert(k);
  }
  // removed nodes are excluded from the change set, so they will never be treated as a changed
  // ancestor
  for (k, change) in payload_change.iter_key_value() {
    if change.is_removed() {
      changed.remove(&k);
      derive.remove(k);
    } else {
      changed.insert(k);
    }
  }

  // for non changed node on the walked path, None means it's on the path that currently being
  // walked, Some(has_changed_ancestor) means the result is resolved.
  let mut walked = FastHashMap::<K, Option<bool>>::default();
  let mut dirty_children = FastHashMap::<K, Vec<K>>::default();
  let mut update_roots = Vec::new();
  let mut path = Vec::new();

  for change in &changed {
    let mut current = change.clone();
    let has_changed_ancestor = loop {
      let Some(parent) = connectivity_source.access(&current) else {
        break false;
      };
      if changed.contains(&parent) {
        dirty_children.entry(parent).or_default().push(current);
        break true;
      }
      match walked.get(&parent) {
        Some(Some(has_changed_ancestor)) => {
          let has_changed_ancestor = *has_changed_ancestor;
          if has_changed_ancestor {
            dirty_children.entry(parent).or_default().push(current);
          }
          break has_changed_ancestor;
        }
        Some(None) => panic!("cycle detected in tree connectivity, at {parent:?}"),
        None => {}
      }
      walked.insert(parent.clone(), None);
      path.push((parent.clone(), current));
      current = parent;
    };

    for (parent, child) in path.drain(..) {
      if has_changed_ancestor {
        dirty_children
          .entry(parent.clone())
          .or_default()
          .push(child);
      }
      walked.insert(parent, Some(has_changed_ancestor));
    }

    if !has_changed_ancestor {
      update_roots.push(change.clone());
    }
  }

  // the stack item is (node, parent, force_recompute)
  let mut stack = Vec::new();
  let mut visited_changed_count = 0;
  for root in update_roots {
    let parent = connectivity_source.access(&root);
    stack.push((root, parent, true));

    while let Some((node, parent, force_recompute)) = stack.pop() {
      let is_changed = changed.contains(&node);
      if is_changed {
        visited_changed_count += 1;
      }

      let derive_changed = if force_recompute || is_changed {
        let parent_derive = parent.map(|parent| {
          derive
            .get_current(parent)
            .expect("parent derive result missing, the connectivity may be dangling")
        });
        let payload = payload_source
          .access(&node)
          .expect("payload missing for alive node");
        let new_derive = derive_logic(&payload, parent_derive);

        if derive.get_current(node.clone()) == Some(&new_derive) {
          false
        } else {
          derive.set_value(node.clone(), new_derive);
          true
        }
      } else {
        false
      };

      if derive_changed {
        if let Some(children) = connectivity_source_rev.access_multi(&node) {
          for child in children {
            stack.push((child, Some(node.clone()), true));
          }
        }
      } else if let Some(children) = dirty_children.remove(&node) {
        for child in children {
          stack.push((child, Some(node.clone()), false));
        }
      }
    }
  }

  // changed nodes on a cycle can not be reached from any update root
  assert_eq!(
    visited_changed_count,
    changed.len(),
    "some changed nodes are not reachable from tree roots, the tree connectivity may contain cycles"
  );
}

#[cfg(test)]
mod test {
  use std::sync::atomic::{AtomicUsize, Ordering};

  use super::*;

  #[derive(Default)]
  struct TreeTester {
    tree_value: FastHashMap<usize, i32>,
    delta: FastHashMap<usize, ValueChange<i32>>,

    tree_connectivity: FastHashMap<usize, usize>,
    tree_connectivity_delta: FastHashMap<usize, ValueChange<usize>>,
    rev_relation: FastHashMap<usize, FastHashSet<usize>>,

    tree_derive_value: FastHashMap<usize, i32>,
    tree_derive_delta: FastHashMap<usize, ValueChange<i32>>,
  }

  impl TreeTester {
    fn test(
      &mut self,
      derive_logic: impl Fn(&i32, Option<&i32>) -> i32 + Send + Sync + 'static + Copy,
      mutator: impl FnOnce(
        &mut dyn QueryLikeMutateTarget<usize, i32>,
        &mut dyn QueryLikeMutateTarget<usize, usize>,
      ),
      asserter: impl FnOnce(&Self),
    ) {
      let mut value_mutator = QueryMutationCollector {
        delta: &mut self.delta,
        target: &mut self.tree_value,
      };

      let mut connectivity_mutator = QueryMutationCollector {
        delta: &mut self.tree_connectivity_delta,
        target: &mut self.tree_connectivity,
      };
      mutator(&mut value_mutator, &mut connectivity_mutator);

      bookkeeping_hash_relation(&mut self.rev_relation, &self.tree_connectivity_delta);

      let mut derive_mutator = QueryMutationCollector {
        delta: &mut self.tree_derive_delta,
        target: &mut self.tree_derive_value,
      };

      compute_tree_derive::<usize, i32>(
        &mut derive_mutator,
        derive_logic,
        &self.tree_value,
        &self.delta,
        &self.tree_connectivity,
        &self.rev_relation,
        &self.tree_connectivity_delta,
      );

      asserter(self);

      self.delta.clear();
      self.tree_connectivity_delta.clear();
      self.tree_derive_delta.clear();
    }

    fn derive_of(&self, k: usize) -> i32 {
      *self.tree_derive_value.get(&k).unwrap()
    }
  }

  fn sum(v: &i32, p: Option<&i32>) -> i32 {
    *v + p.cloned().unwrap_or(0)
  }

  #[test]
  fn test_tree() {
    let mut tester = TreeTester::default();

    tester.test(
      sum,
      |value, connect| {
        // create three nodes
        value.set_value(0, 2);
        value.set_value(1, 1);
        value.set_value(2, 2);
        // add node 1, 2 as 0's children
        connect.set_value(1, 0);
        connect.set_value(2, 0);
      },
      |t| {
        assert_eq!(t.tree_derive_delta.len(), 3);
        assert_eq!(t.derive_of(0), 2);
        assert_eq!(t.derive_of(1), 3);
        assert_eq!(t.derive_of(2), 4);
      },
    );

    tester.test(
      sum,
      |value, connect| {
        value.set_value(3, 1);
        value.set_value(4, 1);
        value.set_value(5, 1);
        connect.set_value(3, 1);
        connect.set_value(4, 3);
        connect.set_value(5, 4);
      },
      |t| {
        assert_eq!(t.tree_derive_delta.len(), 3);
        assert_eq!(t.derive_of(3), 4);
        assert_eq!(t.derive_of(4), 5);
        assert_eq!(t.derive_of(5), 6);
      },
    );

    tester.test(
      sum,
      |value, _connect| {
        value.set_value(1, 2);
        value.set_value(3, 2);
      },
      |t| {
        assert_eq!(t.tree_derive_delta.len(), 4);
        assert_eq!(t.derive_of(1), 4);
        assert_eq!(t.derive_of(3), 6);
        assert_eq!(t.derive_of(4), 7);
        assert_eq!(t.derive_of(5), 8);
      },
    );

    tester.test(
      sum,
      |value, connect| {
        value.set_value(1, 3);
        value.set_value(3, 1);

        value.set_value(6, 0);
        connect.set_value(6, 5);
      },
      |t| {
        assert_eq!(t.tree_derive_delta.len(), 2);
        assert_eq!(t.derive_of(1), 5);
        assert_eq!(t.derive_of(3), 6); // not changed
        assert_eq!(t.derive_of(6), 8);
      },
    );

    // reparent the 3-4-5-6 chain from 1 to 2
    tester.test(
      sum,
      |_value, connect| {
        connect.set_value(3, 2);
      },
      |t| {
        assert_eq!(t.tree_derive_delta.len(), 4);
        assert_eq!(t.derive_of(3), 5);
        assert_eq!(t.derive_of(4), 6);
        assert_eq!(t.derive_of(5), 7);
        assert_eq!(t.derive_of(6), 7);
      },
    );

    // detach 4 to become a root
    tester.test(
      sum,
      |_value, connect| {
        connect.remove(4);
      },
      |t| {
        assert_eq!(t.tree_derive_delta.len(), 3);
        assert_eq!(t.derive_of(4), 1);
        assert_eq!(t.derive_of(5), 2);
        assert_eq!(t.derive_of(6), 2);
      },
    );

    // remove the subtree of 4
    tester.test(
      sum,
      |value, connect| {
        for k in [4, 5, 6] {
          value.remove(k);
          connect.remove(k);
        }
      },
      |t| {
        assert_eq!(t.tree_derive_delta.len(), 3);
        for k in [4, 5, 6] {
          assert!(t.tree_derive_delta.get(&k).unwrap().is_removed());
          assert!(!t.tree_derive_value.contains_key(&k));
        }
      },
    );
  }

  static RECOMPUTE_COUNT: AtomicUsize = AtomicUsize::new(0);

  /// if the changed ancestor's derive result is not changed, only the path down to the changed
  /// descendant should be visited, and the non changed nodes on the path should not be recomputed
  #[test]
  fn test_tree_skip_unchanged_path() {
    fn counted_max(v: &i32, p: Option<&i32>) -> i32 {
      RECOMPUTE_COUNT.fetch_add(1, Ordering::Relaxed);
      (*v).max(p.cloned().unwrap_or(i32::MIN))
    }

    let mut tester = TreeTester::default();

    // 0 is the root with a big value, 1 is its child, 1 has children 10..20, 10 has child 100
    tester.test(
      counted_max,
      |value, connect| {
        value.set_value(0, 100);
        value.set_value(1, 0);
        connect.set_value(1, 0);
        for i in 10..20 {
          value.set_value(i, 0);
          connect.set_value(i, 1);
        }
        value.set_value(100, 0);
        connect.set_value(100, 10);
      },
      |t| assert_eq!(t.derive_of(100), 100),
    );

    RECOMPUTE_COUNT.store(0, Ordering::Relaxed);
    tester.test(
      counted_max,
      |value, _connect| {
        value.set_value(0, 50); // still bigger than all descendants
        value.set_value(100, 1);
      },
      |t| {
        assert_eq!(t.tree_derive_delta.len(), 13);
        assert_eq!(t.derive_of(0), 50);
        assert_eq!(t.derive_of(100), 50);
      },
    );
    // the root's derive result is changed, so the whole tree is recomputed exactly once
    assert_eq!(RECOMPUTE_COUNT.load(Ordering::Relaxed), 13);

    RECOMPUTE_COUNT.store(0, Ordering::Relaxed);
    tester.test(
      counted_max,
      |value, _connect| {
        value.set_value(1, 10); // smaller than parent, derive not changed
        value.set_value(100, 2);
      },
      |t| {
        assert_eq!(t.tree_derive_delta.len(), 0);
        assert_eq!(t.derive_of(100), 50);
      },
    );
    // only 1 and 100 are recomputed, 10 is on the dirty path but not recomputed, 11..20 are not
    // visited
    assert_eq!(RECOMPUTE_COUNT.load(Ordering::Relaxed), 2);
  }

  #[test]
  #[should_panic(expected = "cycle")]
  fn test_tree_cycle_detect() {
    let mut tester = TreeTester::default();
    tester.test(
      sum,
      |value, connect| {
        value.set_value(0, 1);
        value.set_value(1, 1);
        value.set_value(2, 1);
        connect.set_value(1, 2);
        connect.set_value(2, 1);
      },
      |_| {},
    );
  }

  #[test]
  fn test_tree_randomized() {
    struct Rng(u64);
    impl Rng {
      fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
      }
      fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
      }
    }

    // non commutative, so wrong evaluation order can be caught
    fn logic(v: &i32, p: Option<&i32>) -> i32 {
      v.wrapping_mul(31).wrapping_add(p.cloned().unwrap_or(7))
    }

    fn full_compute(t: &TreeTester) -> FastHashMap<usize, i32> {
      fn compute(t: &TreeTester, k: usize, result: &mut FastHashMap<usize, i32>) -> i32 {
        if let Some(v) = result.get(&k) {
          return *v;
        }
        let parent = t.tree_connectivity.get(&k).map(|p| compute(t, *p, result));
        let v = logic(t.tree_value.get(&k).unwrap(), parent.as_ref());
        result.insert(k, v);
        v
      }
      let mut result = FastHashMap::default();
      for k in t.tree_value.keys() {
        compute(t, *k, &mut result);
      }
      result
    }

    fn is_ancestor_or_self(
      connectivity: &FastHashMap<usize, usize>,
      ancestor: usize,
      mut node: usize,
    ) -> bool {
      loop {
        if node == ancestor {
          return true;
        }
        match connectivity.get(&node) {
          Some(parent) => node = *parent,
          None => return false,
        }
      }
    }

    let mut rng = Rng(0x2545F4914F6CDD1D);
    let mut tester = TreeTester::default();
    let mut next_id = 0;

    for _ in 0..300 {
      let previous = tester.tree_derive_value.clone();

      tester.test(
        logic,
        |value, connect| {
          for _ in 0..1 + rng.below(8) {
            let op = rng.below(6);
            let alive: Vec<_> = (0..next_id)
              .filter(|k| value.get_current(*k).is_some())
              .collect();
            let pick = |rng: &mut Rng| alive[rng.below(alive.len())];

            if alive.len() < 3 || op == 0 {
              let k = next_id;
              next_id += 1;
              value.set_value(k, rng.below(5) as i32);
              if !alive.is_empty() && rng.below(4) != 0 {
                connect.set_value(k, pick(&mut rng));
              }
              continue;
            }

            match op {
              1 => {
                let k = pick(&mut rng);
                value.set_value(k, rng.below(5) as i32);
              }
              2 => {
                let k = pick(&mut rng);
                let new_parent = pick(&mut rng);
                let connectivity_snapshot: FastHashMap<_, _> = (0..next_id)
                  .filter_map(|k| connect.get_current(k).map(|p| (k, *p)))
                  .collect();
                if !is_ancestor_or_self(&connectivity_snapshot, k, new_parent) {
                  connect.set_value(k, new_parent);
                }
              }
              3 => {
                let k = pick(&mut rng);
                connect.remove(k);
              }
              _ => {
                // remove a leaf
                let k = pick(&mut rng);
                let has_child = (0..next_id).any(|c| connect.get_current(c) == Some(&k));
                if !has_child {
                  value.remove(k);
                  connect.remove(k);
                }
              }
            }
          }
        },
        |t| {
          let expect = full_compute(t);
          assert_eq!(t.tree_derive_value, expect);

          let mut expect_delta = FastHashMap::default();
          for (k, v) in &expect {
            match previous.get(k) {
              Some(old) if old == v => {}
              old => {
                expect_delta.insert(*k, ValueChange::Delta(*v, old.cloned()));
              }
            }
          }
          for (k, old) in &previous {
            if !expect.contains_key(k) {
              expect_delta.insert(*k, ValueChange::Remove(*old));
            }
          }
          assert_eq!(t.tree_derive_delta, expect_delta);
        },
      );
    }
  }
}
