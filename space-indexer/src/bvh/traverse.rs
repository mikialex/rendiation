use super::{FlattenBVH, FlattenBVHNode, BVHBounding};

impl<B: BVHBounding> FlattenBVH<B> {

  /// reused_history_stack is a preallocate stack to avoid too frequent allocation
  pub fn traverse(
    &self,
    mut leaf_visitor: impl FnMut(&FlattenBVHNode<B>) -> bool,
    mut enter_visitor: impl FnMut(&FlattenBVHNode<B>) -> bool,
    reused_history_stack: &mut Vec<usize>,
  ) {

    reused_history_stack.clear();
    reused_history_stack.push(0);

    while let Some(node_to_visit_index) = reused_history_stack.pop() {
      let node = &self.nodes[node_to_visit_index];
      if enter_visitor(node) {
        if node.is_leaf() {
          leaf_visitor(node);
        } else {
          reused_history_stack.push(node.right_child_offset().unwrap());
          reused_history_stack.push(node.left_child_offset().unwrap());
        }
      }
    }
  }
}
