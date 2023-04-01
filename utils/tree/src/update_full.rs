use crate::*;

pub trait HierarchyDerived {
  type Source;

  fn compute_hierarchy(self_source: &Self::Source, parent_derived: Option<&Self>) -> Self;
}

pub struct ComputedDerivedTree<'a, T: HierarchyDerived> {
  pub source: &'a TreeCollection<T::Source>,
  pub computed: Vec<Option<T>>,
}

impl<'a, T: HierarchyDerived> ComputedDerivedTree<'a, T> {
  pub fn compute_from(source: &'a TreeCollection<T::Source>) -> Self {
    let mut computed = Vec::with_capacity(source.capacity());
    for (handle, node) in &source.nodes.data {
      if node.first_child.is_none() {
        let node = source.create_node_ref(handle);
        node.traverse(&mut |node| {
          let index = node.node.handle().index();
          while computed.len() < index {
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

    Self { source, computed }
  }

  pub fn get_computed(&self, node: TreeNodeHandle<T::Source>) -> &T {
    self.computed[node.index()].as_ref().unwrap()
  }
}
