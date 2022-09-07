#[cfg(test)]
use crate::bst::{BSTTreeNodeRef, Oc};
#[cfg(test)]
use rendiation_abstract_tree::AbstractTree;
#[cfg(test)]
use std::collections::HashSet;
#[cfg(test)]
use std::ops::Range;

#[cfg(test)]
fn print(prefix: &String, name: String, node: &BSTTreeNodeRef<Oc, 8, 3>) {
  if !node.node.primitive_range.is_empty() {
    println!(
      "{}+- {}, primitive_index range [{}, {})",
      prefix, name, node.node.primitive_range.start, node.node.primitive_range.end,
    );
  } else {
    println!("{}+- {}, empty", prefix, name);
  }
}

#[cfg(test)]
fn print_crossed(prefix: &String, node: &BSTTreeNodeRef<Oc, 8, 3>, end: usize) {
  println!(
    "{}+- crossed, primitive_index range [{}, {})",
    prefix, node.node.primitive_range.start, end,
  );
}

#[cfg(test)]
fn visit(prefix: &String, name: String, is_last: bool, node: &BSTTreeNodeRef<Oc, 8, 3>) {
  print(prefix, name, node);
  let child_prefix = if !is_last {
    format!("{}|  ", prefix)
  } else {
    format!("{}   ", prefix)
  };

  let mut index: usize = 0;
  let child_count = node.children_count();

  node.visit_children(|child| {
    if index == 0 && child.node.primitive_range.start > node.node.primitive_range.start {
      print_crossed(&child_prefix, node, node.node.primitive_range.start + 1);
    }

    let is_last = child_count == index + 1;
    visit(&child_prefix, format!("child {}", index), is_last, child);

    index += 1;
  });
}

#[test]
pub fn test_bst_build() {
  use super::OcTree;
  use crate::utils::*;

  const COUNT: usize = 32;
  let boxes = generate_boxes_in_space(COUNT, 100., 1.);
  let tree = OcTree::new(
    boxes.iter().cloned(),
    &TreeBuildOption {
      bin_size: 4,
      max_tree_depth: 10,
    },
  );

  let root = tree.create_node_ref(0);
  visit(&"".into(), "root".into(), true, &root);

  // test tree
  assert!(root.node.child.is_some());
  let mut ranges = Vec::<Range<usize>>::new();
  root.visit_children(|child| {
    ranges.push(child.node.primitive_range.clone());
  });
  assert_eq!(
    ranges,
    vec!(1..1, 1..4, 4..12, 12..15, 15..20, 20..23, 23..27, 27..32)
  );

  // sorted_primitive_index not corrupted
  assert_eq!(
    COUNT,
    HashSet::<usize>::from_iter(tree.sorted_primitive_index.iter().cloned()).len()
  );
}
