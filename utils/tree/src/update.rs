use futures::{Stream, StreamExt};
use incremental::IncrementalBase;
use reactive::SignalStreamExt;

use crate::*;

pub trait HierarchyDepend: Sized {
  fn update_by_parent(&mut self, parent: &Self);
}

impl<T: HierarchyDepend> SharedTreeCollection<T> {
  pub fn update(&self, root: TreeNodeHandle<T>) {
    let mut nodes = self.inner.write().unwrap();
    nodes.traverse_mut_pair(root, |parent, this| {
      let parent = parent.data();
      let node_data = this.data_mut();
      node_data.update_by_parent(parent)
    });
  }
}

pub trait IncrementalHierarchyDepend: HierarchyDepend + IncrementalBase {
  fn check_delta_affects_hierarchy(delta: &Self::Delta) -> bool;
}

/// The default value is the none parent case
pub trait HierarchyDerived: Default + IncrementalBase {
  type Source: IncrementalBase;
  type HierarchyDirtyMark: HierarchyDirtyMark;

  /// for any delta of source, check if it will have hierarchy effect
  fn filter_hierarchy_change(
    change: &<Self::Source as IncrementalBase>::Delta,
  ) -> Option<Self::HierarchyDirtyMark>;

  fn hierarchy_update(&self, parent: &Self);
}

pub trait HierarchyDirtyMark: PartialEq {
  const ALL: Self;
  const NONE: Self;
  fn contains(&self, mark: &Self) -> bool;
  fn intersects(&self, mark: &Self) -> bool;
  fn insert(&mut self, mark: &Self);
}

struct DerivedData<T: HierarchyDerived> {
  data: T,
  /// all sub tree change or together includes self
  /// this tag is for skip tree branch updating
  sub_tree_dirty_mark_any: T::HierarchyDirtyMark,

  /// all sub tree change and together includes self.
  /// this tag is for skipping tree dirty marking
  sub_tree_dirty_mark_all: T::HierarchyDirtyMark,
}

pub struct TreeHierarchyDerivedSystem<T: HierarchyDerived> {
  storage: TreeCollection<DerivedData<T>>,
}

enum DirtyMarkingTask {
  SubTree,
}

impl<T> TreeHierarchyDerivedSystem<T>
where
  T: HierarchyDerived,
  T::Source: IncrementalBase,
{
  #[allow(unused_must_use)]
  pub fn new(
    tree: &SharedTreeCollection<T::Source>,
    tree_delta: impl Stream<Item = SharedTreeMutation<T::Source>>,
  ) -> Self {
    let tree = Arc::new(RwLock::new(TreeCollection::<T>::default()));

    tree_delta
      // mark stage
      // do dirty marking, return if should trigger hierarchy change, and the update root
      .filter_map(|delta: SharedTreeMutation<T::Source>| {
        let derived_tree = tree.write().unwrap();
        async {
          match delta {
            // simply create the default derived. insert into derived tree.
            // we don't care the returned handle, as we assume they are allocated in the same position
            // in the original tree.
            SharedTreeMutation::Create { .. } => {
              derived_tree.create_node(Default::default());
              None
            }
            // do pair remove in derived tree
            SharedTreeMutation::Delete(handle) => {
              derived_tree.delete_node(derived_tree.recreate_handle(handle.index()));
              None
            }
            // check if have any hierarchy effect
            // for children, do traverse mark dirty all-mark, skip if all-mark contains new dirty
            // for parent chain, do parent chain traverse mark dirty any-mark,
            SharedTreeMutation::Mutate { node, delta } => {
              let handle = derived_tree.recreate_handle(node.index());
              if let Some(dirty_mark) = T::filter_hierarchy_change(&delta) {
                mark_sub_tree_full_change(&mut derived_tree, node, dirty_mark)
              }
              None
            }
            // like update, and we will emit full dirty change
            SharedTreeMutation::Attach {
              parent_target,
              node,
            } => {
              let parent_target = derived_tree.recreate_handle(parent_target.index());
              let node = derived_tree.recreate_handle(node.index());
              derived_tree.node_add_child_by(parent_target, node);
              mark_sub_tree_full_change(
                &mut derived_tree,
                node,
                <T as HierarchyDerived>::HierarchyDirtyMark::ALL,
              );
              None
            }
            // ditto
            SharedTreeMutation::Detach { node } => {
              derived_tree.node_detach_parent(derived_tree.recreate_handle(node.index()));
              mark_sub_tree_full_change(
                &mut derived_tree,
                node,
                <T as HierarchyDerived>::HierarchyDirtyMark::ALL,
              );
              None
            }
          }
        }
      })
      .buffered_unbound()
      .map(|_| {
        // this allocation can not removed, but could we calculate correct capacity?
        let derived_updates = Vec::new();
        // do full tree traverse check, emit all real update as stream
      })
      .flatten();

    Self {
      storage: Default::default(),
    }
  }
}

fn mark_sub_tree_full_change<T: HierarchyDerived>(
  tree: &mut TreeCollection<DerivedData<T>>,
  change_node: TreeNodeHandle<DerivedData<T>>,
  dirty_mark: T::HierarchyDirtyMark,
) {
  tree.traverse_mut_pair(node, |node, _| {
    //
  });
}
