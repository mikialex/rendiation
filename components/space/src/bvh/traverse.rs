use super::{BVHBounding, FlattenBVH, FlattenBVHNode};

impl<B: BVHBounding> FlattenBVH<B> {
  /// reused_history_stack is a preallocate stack to avoid too frequent allocation
  pub fn traverse(
    &self,
    mut branch_enter_visitor: impl FnMut(&FlattenBVHNode<B>) -> bool,
    mut leaf_visitor: impl FnMut(&FlattenBVHNode<B>),
  ) {
    let mut stack = Vec::new(); // todo estimate depth for allocation
    stack.push(0);

    while let Some(node_to_visit_index) = stack.pop() {
      let node = &self.nodes[node_to_visit_index];
      if branch_enter_visitor(node) {
        if node.is_leaf() {
          leaf_visitor(node);
        } else {
          stack.push(node.right_child_offset().unwrap());
          stack.push(node.left_child_offset().unwrap());
        }
      }
    }
  }
}
