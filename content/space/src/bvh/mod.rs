mod apply;
mod node;
mod strategy;

mod test;
use std::iter::FromIterator;

pub use node::*;
use rendiation_abstract_tree::NextTraverseVisit;
use rendiation_geometry::SolidEntity;
pub use strategy::*;
pub use test::*;

use crate::{
  utils::{bounding_from_build_source, BuildPrimitive, CenterAblePrimitive, TreeBuildOption},
  AbstractTreeNode,
};

pub trait BVHBounding:
  Sized + Copy + FromIterator<Self> + CenterAblePrimitive + SolidEntity<f32, 3>
{
  type AxisType: Copy;
  fn get_partition_axis(&self) -> Self::AxisType;
}

pub struct FlattenBVH<B: BVHBounding> {
  pub nodes: Vec<FlattenBVHNode<B>>,
  pub sorted_primitive_index: Vec<usize>,
}

#[derive(Clone)]
pub struct BVHTreeNodeRef<'a, B: BVHBounding> {
  pub tree: &'a FlattenBVH<B>,
  pub node: &'a FlattenBVHNode<B>,
}

impl<'a, B: BVHBounding> AbstractTreeNode for BVHTreeNodeRef<'a, B> {
  fn visit_children(&self, mut visitor: impl FnMut(&Self)) {
    if let Some(n) = self.node.left_child_offset() {
      visitor(&self.tree.create_node_ref(n))
    }
    if let Some(n) = self.node.right_child_offset() {
      visitor(&self.tree.create_node_ref(n))
    }
  }
  fn children_count(&self) -> usize {
    if self.has_children() {
      2
    } else {
      0
    }
  }
  fn has_children(&self) -> bool {
    self.node.left_child_offset().is_some()
  }
}

impl<B: BVHBounding> FlattenBVH<B> {
  pub fn new<S: BVHBuildStrategy<B>>(
    source: impl Iterator<Item = B>,
    strategy: &mut S,
    option: &TreeBuildOption,
  ) -> Self {
    // prepare build source;
    let (mut index_list, primitives): (Vec<usize>, Vec<BuildPrimitive<B>>) = source
      .enumerate()
      .map(|(i, b)| (i, BuildPrimitive::new(b)))
      .unzip();

    // prepare root
    let root_bbox =
      bounding_from_build_source(&index_list, primitives.as_slice(), 0..index_list.len());

    let mut nodes = vec![FlattenBVHNode::new(root_bbox, 0..index_list.len(), 0)];

    // build
    strategy.build(option, &primitives, &mut index_list, &mut nodes, 0);

    Self {
      nodes,
      sorted_primitive_index: index_list,
    }
  }

  fn create_node_ref(&self, index: usize) -> BVHTreeNodeRef<B> {
    BVHTreeNodeRef {
      tree: self,
      node: &self.nodes[index],
    }
  }

  pub fn sorted_primitive_index(&self) -> &Vec<usize> {
    &self.sorted_primitive_index
  }

  pub fn traverse(
    &self,
    mut branch_enter_visitor: impl FnMut(&FlattenBVHNode<B>) -> bool,
    mut leaf_visitor: impl FnMut(&FlattenBVHNode<B>) -> bool,
  ) {
    let root = self.create_node_ref(0);
    root.traverse_by_branch_leaf(
      |n| {
        if branch_enter_visitor(n.node) {
          NextTraverseVisit::VisitChildren
        } else {
          NextTraverseVisit::SkipChildren
        }
      },
      |n| leaf_visitor(n.node),
    )
  }
}
