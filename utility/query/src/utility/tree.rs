use rendiation_abstract_tree::*;

use crate::*;

pub fn compute_tree_derive<K: CKey, T: CValue>(
  derive_view: &mut FastHashMap<K, T>,
  derive_logic: impl Fn(&T, Option<&T>) -> T + Send + Sync + 'static + Copy,
  payload_source: impl Query<Key = K, Value = T>,
  payload_change: impl Query<Key = K, Value = ValueChange<T>>,
  connectivity_source: impl Query<Key = K, Value = K>,
  connectivity_source_rev: impl MultiQuery<Key = K, Value = K>,
  connectivity_change: impl Query<Key = K, Value = ValueChange<K>>,
) -> FastHashMap<K, ValueChange<T>> {
  // step1: find the update root
  // we have a change set as input. change set is the composition of the connectivity change
  // and the payload change. for each item in change set, we recursively check it parent if any
  // of the parent exist in the change set, if not, we have a update root

  let mut update_roots = FastHashSet::default();

  let payload_change_range = payload_change.iter_key_value().map(|(k, _)| k);

  let connectivity_change_range = connectivity_change
    .iter_key_value()
    .map(|(k, _)| k)
    .filter(|k| payload_change.access(k).is_none()); // remove the payload_change_range part

  let is_in_change_set = |k| payload_change.contains(&k) || connectivity_change.contains(&k);

  // if we have a very branchy tree, and only root and leaves contains change, it's hard to
  // find the update root is the tree root because early return is not effective. To solve this
  // issue, we may continuously populate the change set by traversing the changeset item's sub
  // tree into change set but it's not a good solution, because it's hard to parallelize  and
  // the most important is that we can assume our tree is not that deep
  for change in payload_change_range.chain(connectivity_change_range) {
    let mut current_check = change.clone();
    loop {
      if let Some(parent) = connectivity_source.access(&current_check) {
        if is_in_change_set(parent.clone()) {
          break;
        }
        current_check = parent.clone();
      } else {
        update_roots.insert(change.clone());
        break;
      }
    }
  }

  // step2: do derive update from all update roots
  // maybe could using some forms of dynamic parallelism
  let mut derive_changes = FastHashMap::default();
  let collector = QueryMutationCollectorPtr {
    delta: &mut derive_changes as *mut _,
    target: derive_view as *mut _,
  };

  let ctx = Ctx {
    derive: collector,
    source: &payload_source,
    connectivity: &connectivity_source_rev,
    parent_connectivity: &connectivity_source,
    derive_logic,
  };
  for root in update_roots {
    let mut root = TreeMutNode {
      phantom: PhantomData,
      idx: root,
      ctx: &ctx,
    };
    root.traverse_pair_subtree_mut(&mut |node, parent| {
      if node.update(parent.as_deref()) {
        NextTraverseVisit::VisitChildren
      } else {
        NextTraverseVisit::SkipChildren
      }
    })
  }

  derive_changes
}

struct QueryMutationCollectorPtr<K, T> {
  delta: *mut FastHashMap<K, ValueChange<T>>,
  target: *mut FastHashMap<K, T>,
}

impl<K: CKey, T: CValue> QueryMutationCollectorPtr<K, T> {
  pub fn get_derive(&self, key: K) -> Option<&T> {
    unsafe { (*self.target).get(&key) }
  }
  pub fn as_mutable(&self) -> impl QueryLikeMutateTarget<K, T> {
    unsafe {
      QueryMutationCollector {
        delta: (&mut *self.delta),
        target: (&mut *self.target),
      }
    }
  }
}

impl<K, T> Clone for QueryMutationCollectorPtr<K, T> {
  fn clone(&self) -> Self {
    Self {
      delta: self.delta,
      target: self.target,
    }
  }
}

#[derive(Clone)]
struct TreeMutNode<'a, K, T, F> {
  phantom: PhantomData<T>,
  idx: K,
  ctx: &'a Ctx<'a, K, T, F>,
}

struct Ctx<'a, K, T, F> {
  derive: QueryMutationCollectorPtr<K, T>,
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
    self.ctx.derive.get_derive(self.idx.clone()).unwrap()
  }
  /// return has actually changed
  pub fn set_derive(&self, d: T) -> bool {
    let p = self
      .ctx
      .derive
      .as_mutable()
      .set_value(self.idx.clone(), d.clone());
    if let Some(p) = p {
      p != d
    } else {
      true
    }
  }
  /// return has actually changed
  pub fn update(&mut self, parent: Option<&Self>) -> bool {
    let parent_derive = parent.map(|parent| parent.get_derive());
    let self_source = self.ctx.source.access_dyn(&self.idx).unwrap();
    self.set_derive((self.ctx.derive_logic)(&self_source, parent_derive))
  }
}

impl<K: CKey, T, F> AbstractTreeMutNode for TreeMutNode<'_, K, T, F> {
  fn visit_children_mut(&mut self, mut visitor: impl FnMut(&mut Self)) {
    if let Some(children) = self.ctx.connectivity.access_multi(&self.idx) {
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
    self
      .ctx
      .parent_connectivity
      .access_dyn(&self.idx)
      .map(|idx| TreeMutNode {
        phantom: PhantomData,
        idx,
        ctx: self.ctx,
      })
  }
}
