use rendiation_math::{Vec2, Vec3};
use rendiation_math_entity::{Box3, Rectangle};
use std::{marker::PhantomData, ops::Range};

pub struct Binary;
pub struct Quad;
pub struct Oc;

pub trait BinarySpaceTree<const N: usize> {
  type Center;
  type Bounding;
}

impl BinarySpaceTree<2> for Binary {
  type Center = f32;
  type Bounding = Range<f32>;
}

impl BinarySpaceTree<4> for Quad {
  type Center = Vec2<f32>;
  type Bounding = Rectangle;
}

impl BinarySpaceTree<8> for Oc {
  type Center = Vec3<f32>;
  type Bounding = Box3;
}

pub struct BSTNode<T: BinarySpaceTree<N>, const N: usize> {
  phantom: PhantomData<T>,
  pub center: T::Center,
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
    let items_count = source.len();
    let (mut index_list, primitives) = source
      .enumerate()
      .map(|(i, b)| (i, BuildPrimitive::new(b)))
      .unzip();

    // prepare root
    let root_bbox = bounding_from_build_source(&index_list, &primitives, 0..items_count);

    let mut nodes = Vec::new();
    nodes.push(BSTNode::new(root_bbox, 0..items_count, 0, 0));

    // build
    strategy.build(&option, &primitives, &mut index_list, &mut nodes);

    Self {
      nodes,
      sorted_primitive_index: index_list,
    }
  }

  pub fn sorted_primitive_index(&self) -> &Vec<usize> {
    &self.sorted_primitive_index
  }
}
