use crate::utils::{BuildPrimitive, CenterAblePrimitive};
use rendiation_math_entity::Box3;
use std::{marker::PhantomData, ops::Range};

pub struct Binary;
pub struct Quad;
pub struct Oc;

pub trait BinarySpaceTree<const N: usize>: Sized {
  type Bounding: CenterAblePrimitive;

  fn create_outer_bounding(
    build_source: &Vec<BuildPrimitive<Self::Bounding>>,
    index_source: &Vec<usize>,
  ) -> Self::Bounding;

  fn prepare_partition(node: &mut BSTNode<Self, N>);

  fn check_primitive_should_in_which_partition(
    primitive: &BuildPrimitive<Self::Bounding>,
  ) -> Option<usize>;

  fn split(
    node: &mut BSTNode<Self, N>,
    build_source: &Vec<BuildPrimitive<Self::Bounding>>,
    index_source: &mut Vec<usize>,
  ) {
    Self::prepare_partition(node);
    index_source
      .get(node.primitive_range.clone())
      .unwrap()
      .iter()
      .map(|&index| &build_source[index].bounding)
      .for_each(|b|{
        if let Some(p) = Self::check_primitive_should_in_which_partition(b) {
          
        }
      })
    todo!()
  }
}

// impl BinarySpaceTree<4> for Quad {
//   type Bounding = Rectangle;
// }

impl BinarySpaceTree<8> for Oc {
  type Bounding = Box3;
  fn create_outer_bounding(
    build_source: &Vec<BuildPrimitive<Self::Bounding>>,
    index_source: &Vec<usize>,
  ) -> Self::Bounding {
    todo!()
  }

  fn check_primitive_should_in_which_partition(
    primitive: &BuildPrimitive<Self::Bounding>,
  ) -> Option<usize> {
    todo!()
  }

  fn prepare_partition(node: &mut BSTNode<Oc, 8>) {
    todo!()
  }
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

pub type BinaryTree = BSTTree<Binary, 2>;
pub type QuadTree = BSTTree<Quad, 4>;
pub type OcTree = BSTTree<Oc, 8>;

pub struct BinarySpaceTreeOption {
  pub max_tree_depth: usize,
  pub bin_size: usize,
}

impl<T: BinarySpaceTree<N>, const N: usize> BSTTree<T, N> {
  pub fn new(
    source: impl ExactSizeIterator<Item = T::Bounding>,
    option: &BinarySpaceTreeOption,
  ) -> Self {
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

    // build
    Self::build(&option, &primitives, &mut index_list, &mut nodes);

    Self {
      nodes,
      sorted_primitive_index: index_list,
    }
  }

  fn build(
    option: &BinarySpaceTreeOption,
    build_source: &Vec<BuildPrimitive<T::Bounding>>,
    index_source: &mut Vec<usize>,
    nodes: &mut Vec<BSTNode<T, N>>,
  ) {
    let building_node = nodes.last().unwrap();

    if building_node.primitive_range.len() <= option.bin_size {
      return;
    }
    if building_node.depth >= option.max_tree_depth {
      return;
    }

    todo!()
  }

  pub fn sorted_primitive_index(&self) -> &Vec<usize> {
    &self.sorted_primitive_index
  }
}
