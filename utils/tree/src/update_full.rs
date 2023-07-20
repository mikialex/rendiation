use std::ops::Deref;

use crate::*;

pub trait HierarchyDerived {
  type Source;

  fn compute_hierarchy(self_source: &Self::Source, parent_derived: Option<&Self>) -> Self;
}

/// we could use lifetime to make sure the source tree not changed when we hold the struct
/// but actually this should did by user
pub struct ComputedDerivedTree<T: HierarchyDerived> {
  pub computed: Vec<Option<T>>,
}

impl<T: HierarchyDerived> ComputedDerivedTree<T> {
  pub fn compute_from<X: Deref<Target = T::Source>>(source: &TreeCollection<X>) -> Self {
    let mut computed = Vec::with_capacity(source.capacity());
    for (handle, node) in &source.nodes.data {
      if node.parent.is_none() {
        let node = source.create_node_ref(handle);
        node.traverse(&mut |node| {
          let index = node.node.handle().index();
          while computed.len() <= index {
            computed.push(None);
          }

          let node_data = node.node.data();
          let node_parent_computed_data = node
            .get_parent()
            .map(|p| computed[p.node.handle().index()].as_ref().unwrap());

          computed[index] = Some(T::compute_hierarchy(node_data, node_parent_computed_data));
        });
      }
    }

    Self { computed }
  }

  pub fn get_computed(&self, index: usize) -> &T {
    self.computed[index].as_ref().unwrap()
  }
}
