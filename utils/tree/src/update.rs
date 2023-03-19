use futures::{Stream, StreamExt};
use incremental::IncrementalBase;

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
pub trait HierarchyDerived: Default {
  type Source: IncrementalBase;
  type HierarchyDirtyMark: HierarchyDirtyMark;

  /// for any delta of source, check if it will have hierarchy effect
  fn filter_hierarchy_change(
    change: &<Self::Source as IncrementalBase>::Delta,
  ) -> Option<Self::HierarchyDirtyMark>;

  fn hierarchy_update(&self, parent: &Self);
}

/// The default case is no dirty
pub trait HierarchyDirtyMark: Default {
  fn contains(&self, mark: &Self) -> bool;
  fn intersects(&self, mark: &Self) -> bool;
  fn insert(&mut self, mark: &Self);
}

struct DerivedData<T: HierarchyDerived> {
  data: T,
  /// all sub tree change or together includes self
  sub_tree_dirty_mark_any: T::HierarchyDirtyMark,

  /// all sub tree change and together includes self
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
  pub fn new(tree_delta: impl Stream<Item = SharedTreeMutation<T::Source>>) -> Self {
    let tree = Arc::new(RwLock::new(TreeCollection::<T>::default()));

    tree_delta
      // mark stage
      .map(|delta: SharedTreeMutation<T::Source>| {
        let tree = tree.write().unwrap();
        match delta {
          SharedTreeMutation::Create(new) => {
            // simply create the default derived. insert derived tree
            // let derived = Default::default();
            // tree.apply(Tree)
          }
          SharedTreeMutation::Delete(handle) => {
            // do pair remove in derived tree
          }
          SharedTreeMutation::Mutate { node, delta } => {
            // check if have hierarchy effect
            // for children, do traverse mark dirty all-mark, skip if all-mark contains new dirty
            // for parent chain, do parent chain traverse mark dirty any-mark,
          }
          SharedTreeMutation::Attach {
            parent_target,
            node,
          } => {
            // like update, and we will emit full dirty change
          }
          SharedTreeMutation::Detach { node } => {
            // like update, and we will emit full dirty change
          }
        };
        // do dirty marking, return if should trigger hierarchy change, and the update root
      })
      .buffered(500) // todo custom buffer? find common update root?
      .map(|_| {
        // do full tree traverse check, emit all real update as stream
      })
      .flatten();

    Self {
      storage: Default::default(),
    }
  }
}
