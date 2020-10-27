use crate::utils::{BuildPrimitive, CenterAblePrimitive};
use rendiation_math_entity::Box3;
use std::{marker::PhantomData, ops::Range};

pub struct Binary;
pub struct Quad;
pub struct Oc;

pub trait BinarySpaceTree<const N: usize> {
  type Bounding: CenterAblePrimitive;
  fn create_outer_bounding(
    build_source: &Vec<BuildPrimitive<Self::Bounding>>,
    index_source: &Vec<usize>,
  ) -> Self::Bounding;
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
  nodes: Vec<BSTNode<T, N>>,
  sorted_primitive_index: Vec<usize>,
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
    todo!()
  }

  pub fn sorted_primitive_index(&self) -> &Vec<usize> {
    &self.sorted_primitive_index
  }
}
