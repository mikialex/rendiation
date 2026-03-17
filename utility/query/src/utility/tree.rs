use rendiation_abstract_tree::*;

use crate::*;

pub fn compute_tree_derive<K: CKey, T: CValue>(
  derive: &mut dyn QueryLikeMutateTarget<K, T>,
  derive_logic: impl Fn(&T, Option<&T>) -> T + Send + Sync + 'static + Copy,
  payload_source: impl Query<Key = K, Value = T>,
  payload_change: impl Query<Key = K, Value = ValueChange<T>>,
  connectivity_source: impl Query<Key = K, Value = K>,
  connectivity_source_rev: impl MultiQuery<Key = K, Value = K>,
  connectivity_change: impl Query<Key = K, Value = ValueChange<K>>,
) {
  // step1: find the update root
  // we have a change set as input. change set is the composition of the connectivity change
  // and the payload change. for each item in change set, we recursively check it parent if any
  // of the parent exist in the change set, if not, we have a update root
  let mut update_roots = FastHashSet::default();
  let mut update_path_not_skipable = FastHashSet::default();

  let payload_change_range =
    payload_change
      .iter_key_value()
      .filter_map(|(k, change)| if change.is_removed() { None } else { Some(k) });

  let connectivity_change_range = connectivity_change
    .iter_key_value()
    .map(|(k, _)| k)
    .filter(|k| payload_change.access(k).is_none()); // remove the payload_change_range part

  let is_in_change_set = |k| payload_change.contains(&k) || connectivity_change.contains(&k);

  // if we have a very branchy tree, and only root and leaves contains change, it's hard to
  // find the update root is the tree root because early return is not effective. To solve this
  // issue, we may continuously populate the change set by traversing the changeset item's sub
  // tree into change set, but it's not a good solution, because it's hard to parallelize
  for change in payload_change_range.chain(connectivity_change_range) {
    let mut current_check = change.clone();
    loop {
      if let Some(parent) = connectivity_source.access(&current_check) {
        if is_in_change_set(parent.clone()) {
          break;
        }
        update_path_not_skipable.insert(parent.clone());
        current_check = parent.clone();
      } else {
        update_roots.insert(change.clone());
        break;
      }
    }
  }

  // step2: do derive update from all update roots
  // this stage should be parallelizable
  let mut ctx = Ctx {
    derive,
    source: &payload_source,
    connectivity: &connectivity_source_rev,
    parent_connectivity: &connectivity_source,
    derive_logic,
  };
  for root in update_roots {
    let mut root = TreeMutNode {
      phantom: PhantomData,
      idx: root,
      ctx: &mut ctx as *mut _,
    };
    root.traverse_pair_subtree_mut(&mut |node, parent| {
      if node.update(parent.as_deref()) {
        NextTraverseVisit::VisitChildren
      } else {
        if update_path_not_skipable.contains(&node.idx) || is_in_change_set(node.idx.clone()) {
          NextTraverseVisit::VisitChildren
        } else {
          NextTraverseVisit::SkipChildren
        }
      }
    })
  }
}

#[test]
fn test_tree() {
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
    pub fn test(
      &mut self,
      mutator: impl FnOnce(
        &mut dyn QueryLikeMutateTarget<usize, i32>,
        &mut dyn QueryLikeMutateTarget<usize, usize>,
      ),
      asserter: impl FnOnce(&FastHashMap<usize, i32>, &FastHashMap<usize, ValueChange<i32>>),
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
        |v, p| *v + p.cloned().unwrap_or(0),
        &self.tree_value,
        &self.delta,
        &self.tree_connectivity,
        &self.rev_relation,
        &self.tree_connectivity_delta,
      );

      asserter(&self.tree_derive_value, &self.tree_derive_delta);

      self.delta.clear();
      self.tree_connectivity_delta.clear();
      self.tree_derive_delta.clear();
    }
  }

  let mut tester = TreeTester::default();

  tester.test(
    |value, connect| {
      // create three nodes
      value.set_value(0, 2);
      value.set_value(1, 1);
      value.set_value(2, 2);
      // add node 1, 2 as 0's children
      connect.set_value(1, 0);
      connect.set_value(2, 0);
    },
    |derive_value, derive_delta| {
      assert_eq!(derive_delta.len(), 3);
      assert_eq!(*derive_value.get(&0).unwrap(), 2);
      assert_eq!(*derive_value.get(&1).unwrap(), 3);
      assert_eq!(*derive_value.get(&2).unwrap(), 4);
    },
  );

  tester.test(
    |value, connect| {
      value.set_value(3, 1);
      value.set_value(4, 1);
      value.set_value(5, 1);
      connect.set_value(3, 1);
      connect.set_value(4, 3);
      connect.set_value(5, 4);
    },
    |derive_value, derive_delta| {
      assert_eq!(derive_delta.len(), 3);
      assert_eq!(*derive_value.get(&3).unwrap(), 4);
      assert_eq!(*derive_value.get(&4).unwrap(), 5);
      assert_eq!(*derive_value.get(&5).unwrap(), 6);
    },
  );

  tester.test(
    |value, _connect| {
      value.set_value(1, 2);
      value.set_value(3, 2);
    },
    |derive_value, derive_delta| {
      assert_eq!(derive_delta.len(), 4);
      assert_eq!(*derive_value.get(&1).unwrap(), 4);
      assert_eq!(*derive_value.get(&3).unwrap(), 6);
      assert_eq!(*derive_value.get(&4).unwrap(), 7);
      assert_eq!(*derive_value.get(&5).unwrap(), 8);
    },
  );

  tester.test(
    |value, connect| {
      value.set_value(1, 3);
      value.set_value(3, 1);

      value.set_value(6, 0);
      connect.set_value(6, 5);
    },
    |derive_value, derive_delta| {
      assert_eq!(derive_delta.len(), 2);
      assert_eq!(*derive_value.get(&1).unwrap(), 5);
      assert_eq!(*derive_value.get(&3).unwrap(), 6); // not changed
      assert_eq!(*derive_value.get(&6).unwrap(), 8);
    },
  );
}

#[derive(Clone)]
struct TreeMutNode<'a, K, T, F> {
  phantom: PhantomData<T>,
  idx: K,
  ctx: *mut Ctx<'a, K, T, F>,
}

struct Ctx<'a, K, T, F> {
  derive: &'a mut dyn QueryLikeMutateTarget<K, T>,
  source: &'a dyn DynQuery<Key = K, Value = T>,
  connectivity: &'a dyn DynMultiQuery<Key = K, Value = K>,
  parent_connectivity: &'a dyn DynQuery<Key = K, Value = K>,
  derive_logic: F,
}

impl<K, T, F> TreeMutNode<'_, K, T, F>
where
  K: CKey,
  T: CValue,
  F: Fn(&T, Option<&T>) -> T + Send + Sync + 'static + Copy,
{
  pub fn get_derive(&self) -> &T {
    let derive = &(unsafe { &mut *self.ctx }).derive;
    derive.get_current(self.idx.clone()).unwrap()
  }
  /// return has actually changed
  pub fn set_derive(&mut self, d: T) -> bool {
    let derive = &mut (unsafe { &mut *self.ctx }).derive;
    let p = derive.set_value(self.idx.clone(), d.clone());
    if let Some(p) = p {
      p != d
    } else {
      true
    }
  }
  /// return has actually changed
  pub fn update(&mut self, parent: Option<&Self>) -> bool {
    let ctx = unsafe { &*self.ctx };
    let parent_derive = parent.map(|parent| parent.get_derive());
    let self_source = ctx.source.access_dyn(&self.idx).unwrap();
    self.set_derive((ctx.derive_logic)(&self_source, parent_derive))
  }
}

impl<K: CKey, T, F> AbstractTreeMutNode for TreeMutNode<'_, K, T, F> {
  fn visit_children_mut(&mut self, mut visitor: impl FnMut(&mut Self)) {
    let ctx = unsafe { &*self.ctx };
    if let Some(children) = ctx.connectivity.access_multi(&self.idx) {
      for idx in children {
        visitor(&mut TreeMutNode {
          phantom: PhantomData,
          idx,
          ctx: self.ctx,
        });
      }
    }
  }
}

impl<K: CKey, T, F> AbstractParentAddressableMutTreeNode for TreeMutNode<'_, K, T, F> {
  fn get_parent_mut(&mut self) -> Option<Self> {
    let ctx = unsafe { &*self.ctx };
    ctx
      .parent_connectivity
      .access_dyn(&self.idx)
      .map(|idx| TreeMutNode {
        phantom: PhantomData,
        idx,
        ctx: self.ctx,
      })
  }
}
