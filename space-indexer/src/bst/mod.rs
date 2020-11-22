use crate::utils::{BuildPrimitive, CenterAblePrimitive, TreeBuildOption};
use std::{marker::PhantomData, ops::Range};

pub mod apply;
pub use apply::*;

pub trait BinarySpaceTree<const N: usize>: Sized {
  type Bounding: CenterAblePrimitive + Default + Copy;

  fn create_outer_bounding(
    build_source: &Vec<BuildPrimitive<Self::Bounding>>,
    index_source: &Vec<usize>,
  ) -> Self::Bounding;

  fn prepare_partition(node: &mut BSTNode<Self, { N }>);

  fn check_primitive_should_in_which_partition(
    primitive: &BuildPrimitive<Self::Bounding>,
  ) -> Option<usize>;

  fn get_sub_space(index: usize) -> Self::Bounding;
}

pub struct BSTNode<T: BinarySpaceTree<N>, const N: usize> {
  phantom: PhantomData<T>,
  pub bounding: T::Bounding,
  pub primitive_range: Range<usize>,
  pub depth: usize,
  pub self_index: usize,
  pub child: Option<[usize; N]>,
}

pub struct BSTTree<T: BinarySpaceTree<N>, const N: usize> {
  pub nodes: Vec<BSTNode<T, N>>,
  pub sorted_primitive_index: Vec<usize>,
}

pub struct BSTTreeBuilder<T: BinarySpaceTree<N>, const N: usize> {
  partitions: Vec<Vec<usize>>,
  ranges: Vec<Range<usize>>,
  crossed: Vec<usize>,
  bounding: Vec<T::Bounding>,
}

impl<T: BinarySpaceTree<N>, const N: usize> BSTTreeBuilder<T, N> {
  fn new(size: usize) -> Self {
    Self {
      partitions: (0..size).map(|_| Vec::new()).collect(),
      ranges: (0..size).map(|_| (0..0)).collect(),
      crossed: Vec::new(),
      bounding: (0..size).map(|_| T::Bounding::default()).collect(),
    }
  }
  fn reset(&mut self) {
    self.partitions.iter_mut().for_each(|p| p.clear());
    self.crossed.clear();
    self
      .bounding
      .iter_mut()
      .enumerate()
      .for_each(|(i, b)| *b = T::get_sub_space(i))
  }
  fn set(&mut self, p: Option<usize>, index: usize) {
    if let Some(p) = p {
      self.partitions[p].push(index)
    } else {
      self.crossed.push(index)
    }
  }
  fn apply_index_source(&self, index_source: &mut [usize]) {
    todo!()
  }
}

pub type BinaryTree = BSTTree<Binary, 2>;
pub type QuadTree = BSTTree<Quad, 4>;
pub type OcTree = BSTTree<Oc, 8>;

impl<T: BinarySpaceTree<N>, const N: usize> BSTTree<T, N> {
  pub fn new(source: impl ExactSizeIterator<Item = T::Bounding>, option: &TreeBuildOption) -> Self {
    // prepare build source;
    let (mut index_list, primitives) = source
      .enumerate()
      .map(|(i, b)| (i, BuildPrimitive::new(b)))
      .unzip();

    // prepare root
    let root_bbox = T::create_outer_bounding(&primitives, &index_list);

    let mut nodes = Vec::new();
    nodes.push(BSTNode {
      phantom: PhantomData,
      bounding: root_bbox,
      primitive_range: 0..index_list.len(),
      depth: 0,
      self_index: 0,
      child: None,
    });

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

  fn build(
    option: &TreeBuildOption,
    build_source: &Vec<BuildPrimitive<T::Bounding>>,
    index_source: &mut Vec<usize>,
    nodes: &mut Vec<BSTNode<T, N>>,
    builder: &mut BSTTreeBuilder<T, N>,
  ) -> usize {
    let (node_index, depth) = {
      let node_index = nodes.len() - 1;
      let node = nodes.last_mut().unwrap();

      if option.should_continue(node.primitive_range.len(), node.depth) {
        return 1;
      }

      builder.reset();
      T::prepare_partition(node);
      index_source
        .get(node.primitive_range.clone())
        .unwrap()
        .iter()
        .map(|&index| (index, &build_source[index]))
        .for_each(|(index, b)| builder.set(T::check_primitive_should_in_which_partition(b), index));
      builder.apply_index_source(index_source.get_mut(node.primitive_range.clone()).unwrap());
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
