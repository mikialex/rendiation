use crate::{
  utils::{bounding_from_build_source, BuildPrimitive, CenterAblePrimitive, TreeBuildOption},
  AbstractTree,
};
use std::{iter::FromIterator, marker::PhantomData, ops::Range};

pub mod apply;
pub mod test;
pub use apply::*;
use rendiation_geometry::ContainAble;

/// The BST tree build source trait
///
pub trait BSTBounding<const D: usize, const N: usize>:
  CenterAblePrimitive + Default + Copy + ContainAble<f32, Self, D> + FromIterator<Self>
{
  /// Check which sub space should the build primitive belong
  fn pre_classify_primitive(&self, p: &BuildPrimitive<Self>) -> usize;

  /// Compute the child space BSTBounding by child index
  #[must_use]
  fn compute_sub_space(&self, index: usize) -> Self;
}

pub trait BinarySpaceTree<const D: usize, const N: usize>: Sized {
  type Bounding: BSTBounding<D, N>;
}

pub struct BSTNode<T: BinarySpaceTree<D, N>, const N: usize, const D: usize> {
  phantom: PhantomData<T>,
  pub bounding: T::Bounding,
  pub primitive_range: Range<usize>,
  pub depth: usize,
  pub self_index: usize,
  pub child: Option<[usize; N]>,
}

pub struct BSTTree<T: BinarySpaceTree<D, N>, const N: usize, const D: usize> {
  pub nodes: Vec<BSTNode<T, N, D>>,
  pub sorted_primitive_index: Vec<usize>,
}

pub struct BSTTreeNodeRef<'a, T, const N: usize, const D: usize>
where
  T: BinarySpaceTree<D, N>,
{
  pub tree: &'a BSTTree<T, N, D>,
  pub node: &'a BSTNode<T, N, D>,
}

impl<'a, T, const N: usize, const D: usize> AbstractTree for BSTTreeNodeRef<'a, T, N, D>
where
  T: BinarySpaceTree<D, N>,
{
  fn visit_children(&self, mut visitor: impl FnMut(&Self)) {
    if let Some(children) = &self.node.child {
      for child in children {
        let child = self.tree.create_node_ref(*child);
        visitor(&child)
      }
    }
  }
  fn children_count(&self) -> usize {
    if self.has_children() {
      N
    } else {
      0
    }
  }
  fn has_children(&self) -> bool {
    self.node.child.is_some()
  }
}

pub struct BSTTreeBuilder<T: BinarySpaceTree<D, N>, const N: usize, const D: usize> {
  partitions: Vec<Vec<usize>>,
  ranges: Vec<Range<usize>>,
  crossed: Vec<usize>,
  bounding: Vec<T::Bounding>,
}

impl<T: BinarySpaceTree<D, N>, const N: usize, const D: usize> BSTTreeBuilder<T, N, D> {
  fn new(size: usize) -> Self {
    Self {
      partitions: (0..size).map(|_| Vec::new()).collect(),
      ranges: (0..size).map(|_| (0..0)).collect(),
      crossed: Vec::new(),
      bounding: (0..size).map(|_| T::Bounding::default()).collect(),
    }
  }
  fn reset(&mut self, all_bounding: T::Bounding) {
    self.partitions.iter_mut().for_each(|p| p.clear());
    self.crossed.clear();
    self
      .bounding
      .iter_mut()
      .enumerate()
      .for_each(|(i, b)| *b = all_bounding.compute_sub_space(i))
  }
  fn classify_primitive(
    &mut self,
    node: &BSTNode<T, N, D>,
    p: &BuildPrimitive<T::Bounding>,
    index: usize,
  ) {
    let preferred_index = node.bounding.pre_classify_primitive(p);
    let preferred_sub_box = &self.bounding[preferred_index];

    if preferred_sub_box.contains(&p.bounding) {
      self.partitions[preferred_index].push(index)
    } else {
      self.crossed.push(index)
    }
  }
  fn apply_index_source(&mut self, index_source: &mut Vec<usize>, range: Range<usize>) {
    let mut start = range.start;
    let ranges = &mut self.ranges;
    self
      .partitions
      .iter()
      .enumerate()
      .for_each(|(index, primitives)| {
        let mut count = 0;
        primitives.iter().for_each(|&i| {
          index_source[start + count] = i;
          count += 1;
        });
        ranges[index] = start..start + count;
        start += count;
      })
  }
}

impl<T: BinarySpaceTree<D, N>, const N: usize, const D: usize> BSTTree<T, N, D> {
  pub fn new(source: impl ExactSizeIterator<Item = T::Bounding>, option: &TreeBuildOption) -> Self {
    // prepare build source;
    let (mut index_list, primitives): (Vec<usize>, Vec<BuildPrimitive<T::Bounding>>) = source
      .enumerate()
      .map(|(i, b)| (i, BuildPrimitive::new(b)))
      .unzip();

    // prepare root
    let root_bbox =
      bounding_from_build_source(&index_list, primitives.as_slice(), 0..index_list.len());

    let mut nodes = vec![BSTNode {
      phantom: PhantomData,
      bounding: root_bbox,
      primitive_range: 0..index_list.len(),
      depth: 0,
      self_index: 0,
      child: None,
    }];

    Self::build(
      option,
      &primitives,
      &mut index_list,
      &mut nodes,
      &mut BSTTreeBuilder::new(N),
    );

    Self {
      nodes,
      sorted_primitive_index: index_list,
    }
  }

  fn create_node_ref(&self, index: usize) -> BSTTreeNodeRef<T, N, D> {
    BSTTreeNodeRef {
      tree: self,
      node: &self.nodes[index],
    }
  }

  fn build(
    option: &TreeBuildOption,
    build_source: &[BuildPrimitive<T::Bounding>],
    index_source: &mut Vec<usize>,
    nodes: &mut Vec<BSTNode<T, N, D>>,
    builder: &mut BSTTreeBuilder<T, N, D>,
  ) -> usize {
    let (node_index, depth) = {
      let node_index = nodes.len() - 1;
      let node = nodes.last_mut().unwrap();

      if !option.should_continue(node.primitive_range.len(), node.depth) {
        return 1;
      }

      builder.reset(node.bounding);
      index_source
        .get(node.primitive_range.clone())
        .unwrap()
        .iter()
        .map(|&index| (index, &build_source[index]))
        .for_each(|(index, b)| builder.classify_primitive(node, b, index));
      builder.apply_index_source(index_source, node.primitive_range.clone());
      (node_index, node.depth)
    };

    let mut child = [0; N];
    let mut offset = 1;
    let ranges = builder.ranges.clone();
    for (i, range) in ranges.iter().enumerate() {
      nodes.push(BSTNode {
        phantom: PhantomData,
        bounding: builder.bounding[i],
        primitive_range: range.clone(),
        depth: depth + 1,
        self_index: nodes.len(),
        child: None,
      });
      child[i] = offset;
      offset += Self::build(option, build_source, index_source, nodes, builder);
    }
    nodes[node_index].child = Some(child);
    offset
  }
}
