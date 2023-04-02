use incremental::*;

use crate::*;
use std::ops::Deref;

#[derive(Incremental, Clone)]
struct TestNode {
  pub value: usize,
}

impl Deref for TestNode {
  type Target = Self;

  fn deref(&self) -> &Self::Target {
    self
  }
}

#[derive(Incremental, Copy, Clone, Default)]
struct TestNodeDerived {
  pub value_sum: usize,
}

impl HierarchyDerived for TestNodeDerived {
  type Source = TestNode;

  fn compute_hierarchy(self_source: &Self::Source, parent_derived: Option<&Self>) -> Self {
    if let Some(parent) = parent_derived {
      Self {
        value_sum: self_source.value + parent.value_sum,
      }
    } else {
      Self {
        value_sum: self_source.value,
      }
    }
  }
}

#[derive(Default, PartialEq)]
struct ValueSumIsDirty(bool);
impl HierarchyDirtyMark for ValueSumIsDirty {
  fn contains(&self, mark: &Self) -> bool {
    self.0 && mark.0
  }

  fn intersects(&self, mark: &Self) -> bool {
    self.0 && mark.0
  }

  fn insert(&mut self, mark: &Self) {
    self.0 = mark.0;
  }

  fn all_dirty() -> Self {
    ValueSumIsDirty(true)
  }
}

impl IncrementalHierarchyDerived for TestNodeDerived {
  type Source = TestNode;

  type DirtyMark = ValueSumIsDirty;

  fn filter_hierarchy_change(
    change: &<Self::Source as IncrementalBase>::Delta,
  ) -> Option<Self::DirtyMark> {
    match change {
      TestNodeDelta::value(_) => Some(ValueSumIsDirty(true)),
    }
  }

  fn hierarchy_update(
    &mut self,
    self_source: &Self::Source,
    parent_derived: Option<&Self>,
    dirty: &Self::DirtyMark,
    mut collect: impl FnMut(Self::Delta),
  ) {
    if dirty.0 {
      if let Some(parent) = parent_derived {
        self.value_sum = self_source.value + parent.value_sum;
        collect(TestNodeDerivedDelta::value_sum(self.value_sum));
      } else {
        self.value_sum = self_source.value;
        collect(TestNodeDerivedDelta::value_sum(self.value_sum));
      }
    }
  }
}

#[test]
fn test_full_update() {
  let mut tree = TreeCollection::default();
  let root = tree.create_node(TestNode { value: 0 });
  let a = tree.create_node(TestNode { value: 3 });
  let b = tree.create_node(TestNode { value: 2 });
  let c = tree.create_node(TestNode { value: 1 });
  let d = tree.create_node(TestNode { value: 10 });
  tree.node_add_child_by(root, a).unwrap();
  tree.node_add_child_by(a, b).unwrap();
  tree.node_add_child_by(a, c).unwrap();

  let derived = ComputedDerivedTree::<TestNodeDerived>::compute_from(&tree);

  let root_derived = derived.get_computed(root.index());
  assert_eq!(root_derived.value_sum, 0);
  let b_derived = derived.get_computed(b.index());
  assert_eq!(b_derived.value_sum, 5);
  let c_derived = derived.get_computed(c.index());
  assert_eq!(c_derived.value_sum, 4);
  let d_derived = derived.get_computed(d.index());
  assert_eq!(d_derived.value_sum, 10);
}

#[test]
fn test_inc_update() {
  let tree = SharedTreeCollection::<ReactiveTreeCollection<TestNode, TestNode>>::default();
  let stream = tree.visit_inner(|t| t.source.listen());
  let mut tree_sys = TreeHierarchyDerivedSystem::<TestNodeDerived>::new(stream, &tree);

  let root = tree.create_new_root(TestNode { value: 0 });
  let a = root.create_child(TestNode { value: 3 });
  let b = a.create_child(TestNode { value: 2 });
  let c = a.create_child(TestNode { value: 1 });
  let d = tree.create_new_root(TestNode { value: 10 });

  tree_sys.maintain();

  let getter =
    |node: ShareTreeNode<ReactiveTreeCollection<TestNode, TestNode>>| -> TestNodeDerived {
      tree_sys.visit_derived_tree(|tree| {
        let handle = tree.recreate_handle(node.raw_handle().index());
        tree.get_node(handle).data().data
      })
    };

  let root_derived = getter(root);
  assert_eq!(root_derived.value_sum, 0);
  let b_derived = getter(b);
  assert_eq!(b_derived.value_sum, 5);
  let c_derived = getter(c);
  assert_eq!(c_derived.value_sum, 4);
  let d_derived = getter(d);
  assert_eq!(d_derived.value_sum, 10);
}
