use rendiation_abstract_tree::*;
use storage::IndexKeptVec;

use crate::*;

pub trait ParentDecideChildren: Sized + Default {
  fn update(self_source: &Self, parent_derive: Option<Self>) -> Self;
}

pub type ReactiveTreeConnectivity = Box<dyn ReactiveOneToManyRelationship<u32, u32>>;
pub type ReactiveTreePayload<T> = Box<dyn ReactiveCollection<u32, T>>;

pub fn tree_payload_derive_by_parent_decide_children<T>(
  connectivity: ReactiveTreeConnectivity,
  payload: ReactiveTreePayload<T>,
) -> impl ReactiveCollection<u32, T>
where
  T: CValue + ParentDecideChildren,
{
  TreeDerivedData {
    data: Default::default(),
    payload_source: payload,
    connectivity_source: connectivity,
  }
}

struct TreeDerivedData<T> {
  /// where the actually derived data stored
  data: Arc<RwLock<IndexKeptVec<T>>>,
  payload_source: ReactiveTreePayload<T>,
  connectivity_source: ReactiveTreeConnectivity,
}

impl<T: CValue + ParentDecideChildren> ReactiveCollection<u32, T> for TreeDerivedData<T> {
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<u32, T> {
    let payload_change = self.payload_source.poll_changes(cx);
    let connectivity_change = self.connectivity_source.poll_changes(cx);

    if payload_change.is_pending() && connectivity_change.is_pending() {
      return Poll::Pending;
    }

    let payload_change = match payload_change {
      Poll::Ready(v) => v,
      Poll::Pending => Box::new(()),
    };
    let connectivity_change = match connectivity_change {
      Poll::Ready(v) => v,
      Poll::Pending => Box::new(()),
    };

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

    let current_connectivity = self.connectivity_source.access();

    let is_in_change_set = |k| payload_change.contains(&k) || connectivity_change.contains(&k);

    // if we have a very branchy tree, and only root and leaves contains change, it's hard to
    // find the update root is the tree root because early return is not effective. To solve this
    // issue, we may continuously populate the change set by traversing the changeset item's sub
    // tree into change set but it's not a good solution, because it's hard to parallelize  and
    // the most important is that we can assume our tree is not that deep
    for change in payload_change_range.chain(connectivity_change_range) {
      let mut current_check = change;
      loop {
        if let Some(parent) = current_connectivity.access(&current_check) {
          if is_in_change_set(parent) {
            break;
          }
          current_check = parent;
        } else {
          update_roots.insert(change);
          break;
        }
      }
    }

    // step2: do derive update from all update roots
    // maybe could using some forms of dynamic parallelism
    let mut derive_tree = self.data.write();
    let mut derive_changes = FastHashMap::default();
    let collector = CollectionMutationCollectorPtr {
      delta: &mut derive_changes as *mut _,
      target: (&mut derive_tree as &mut IndexKeptVec<T>) as *mut _,
    };

    let current_source = self.payload_source.access();
    let current_inv_connectivity = self.connectivity_source.multi_access();
    let ctx = Ctx {
      derive: collector,
      source: current_source.as_ref(),
      connectivity: current_inv_connectivity.as_ref(),
      parent_connectivity: current_connectivity.as_ref(),
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

    if derive_changes.is_empty() {
      Poll::Pending
    } else {
      Poll::Ready(Box::new(derive_changes))
    }
  }

  fn access(&self) -> PollCollectionCurrent<u32, T> {
    Box::new(self.data.make_read_holder())
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.data.write().shrink_to_fit();
    self.payload_source.extra_request(request);
    self.connectivity_source.extra_request(request);
  }
}

struct CollectionMutationCollectorPtr<T> {
  delta: *mut FastHashMap<u32, ValueChange<T>>,
  target: *mut IndexKeptVec<T>,
}

impl<T: CValue> CollectionMutationCollectorPtr<T> {
  pub fn get_derive(&self, key: u32) -> Option<T> {
    unsafe { (*self.target).try_get(key).cloned() }
  }
  pub fn as_mutable(&self) -> impl MutableCollection<u32, T> {
    unsafe {
      CollectionMutationCollector {
        delta: (&mut *self.delta),
        target: (&mut *self.target),
      }
    }
  }
}

impl<T> Clone for CollectionMutationCollectorPtr<T> {
  fn clone(&self) -> Self {
    Self {
      delta: self.delta,
      target: self.target,
    }
  }
}

#[derive(Clone)]
struct TreeMutNode<'a, T> {
  phantom: PhantomData<T>,
  idx: u32,
  ctx: &'a Ctx<'a, T>,
}

struct Ctx<'a, T> {
  derive: CollectionMutationCollectorPtr<T>,
  source: &'a dyn VirtualCollection<u32, T>,
  connectivity: &'a dyn VirtualMultiCollection<u32, u32>,
  parent_connectivity: &'a dyn VirtualCollection<u32, u32>,
}

impl<'a, T> TreeMutNode<'a, T>
where
  T: ParentDecideChildren + CValue,
{
  pub fn get_derive(&self) -> T {
    self.ctx.derive.get_derive(self.idx).unwrap()
  }
  /// return has actually changed
  pub fn set_derive(&self, d: T) -> bool {
    let p = self.ctx.derive.as_mutable().set_value(self.idx, d.clone());
    if let Some(p) = p {
      p == d
    } else {
      true
    }
  }
  /// return has actually changed
  pub fn update(&mut self, parent: Option<&Self>) -> bool {
    let parent_derive = parent.map(|parent| parent.get_derive());
    let self_source = self.ctx.source.access(&self.idx).unwrap();
    self.set_derive(ParentDecideChildren::update(&self_source, parent_derive))
  }
}

impl<'a, T> AbstractTreeMutNode for TreeMutNode<'a, T> {
  fn visit_children_mut(&mut self, mut visitor: impl FnMut(&mut Self)) {
    self.ctx.connectivity.access_multi(&self.idx, &mut |idx| {
      visitor(&mut TreeMutNode {
        phantom: PhantomData,
        idx,
        ctx: self.ctx,
      });
    });
  }
}

impl<'a, T> AbstractParentAddressableMutTreeNode for TreeMutNode<'a, T> {
  fn get_parent_mut(&mut self) -> Option<Self> {
    self
      .ctx
      .parent_connectivity
      .access(&self.idx)
      .map(|idx| TreeMutNode {
        phantom: PhantomData,
        idx,
        ctx: self.ctx,
      })
  }
}
