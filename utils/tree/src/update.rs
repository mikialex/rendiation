use incremental::IncrementalBase;

use crate::*;

// todo impl fine-grained update

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
