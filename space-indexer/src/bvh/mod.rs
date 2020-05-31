mod node;
mod strategy;
mod traverse;

pub use node::*;
use rendiation_math::Vec3;
use rendiation_math_entity::Axis;
use std::{cmp::Ordering, ops::Range};
pub use strategy::*;

pub trait BVHBounding<P>: Sized {
  fn center(&self) -> Vec3<f32>;
  fn from_groups(iter: impl Iterator<Item = Self>) -> Self;
  fn get_partition_axis(&self) -> P;
  fn compare_center(&self, axis: P, other: &Self) -> Ordering;
}

pub trait FlattenBVHBuildSource<P, B: BVHBounding<P>> {
  fn get_items_count(&self) -> usize;
  fn get_items_bounding_box(&self, item_index: usize) -> B;
}

pub struct BVHOption {
  pub max_tree_depth: usize,
  pub bin_size: usize,
}

impl Default for BVHOption {
  fn default() -> Self {
    Self {
      max_tree_depth: 10,
      bin_size: 1,
    }
  }
}

pub struct FlattenBVH<P, B: BVHBounding<P>> {
  nodes: Vec<FlattenBVHNode<B, P>>,
  sorted_primitive_index: Vec<usize>,
  option: BVHOption,
}

impl<P, B: BVHBounding<P>> FlattenBVH<P, B> {
  pub fn new<T: BVHBuildStrategy<P, B>>(source: impl FlattenBVHBuildSource<P, B>) -> Self {
    let option = BVHOption::default();

    // prepare build source;
    let items_count = source.get_items_count();
    let mut index_list: Vec<usize> = (0..items_count).map(|x| x).collect();
    let primitives: Vec<BuildPrimitive<P, B>> = (0..items_count)
      .map(|x| BuildPrimitive::new(source.get_items_bounding_box(x)))
      .collect();

    // prepare root
    let root_bbox = box_from_build_source(&index_list, &primitives, 0..items_count);

    let mut nodes = Vec::new();
    nodes.push(FlattenBVHNode::new(root_bbox, 0..items_count, 0, 0));

    // build
    T::build(&option, &primitives, &mut index_list, &mut nodes);

    Self {
      nodes,
      sorted_primitive_index: index_list,
      option,
    }
  }

  pub fn option(&self) -> &BVHOption {
    &self.option
  }

  pub fn sorted_primitive_index(&self) -> &Vec<usize> {
    &self.sorted_primitive_index
  }
}

fn box_from_build_source<P, B: BVHBounding<P>>(
  index_list: &Vec<usize>,
  primitives: &Vec<BuildPrimitive<P, B>>,
  range: Range<usize>,
) -> B {
  B::from_groups(
    index_list
      .get(range.clone())
      .unwrap()
      .iter()
      .map(|index| primitives[*index].bbox),
  )
}
use std::marker::PhantomData;
pub struct BuildPrimitive<P, B> {
  bbox: B,
  center: Vec3<f32>,
  _phantom_data: PhantomData<P>
}

impl<P, B: BVHBounding<P>> BuildPrimitive<P, B> {
  fn new(bbox: B) -> Self {
    Self {
      bbox,
      center: bbox.center(),
      _phantom_data: PhantomData
    }
  }

  fn compare_center(&self, axis: P, other: &BuildPrimitive<P, B>) -> Ordering {
    self.bbox.compare_center(axis, &other.bbox)
    // match axis {
    //   Axis::X => self.center.x.partial_cmp(&other.center.x).unwrap(),
    //   Axis::Y => self.center.y.partial_cmp(&other.center.y).unwrap(),
    //   Axis::Z => self.center.z.partial_cmp(&other.center.z).unwrap(),
    // }
  }
}
