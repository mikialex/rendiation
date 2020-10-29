use rand::random;
use rendiation_math::Vec3;
use rendiation_math_entity::Box3;

pub trait CenterAblePrimitive {
  type Center;
  fn get_center(&self) -> Self::Center;
}

impl CenterAblePrimitive for Box3 {
  type Center = Vec3<f32>;

  fn get_center(&self) -> Vec3<f32> {
    self.center()
  }
}

pub trait SpacePartitionTreeNode {
  fn contained_item_count(&self) -> usize;
}

pub struct TreeBuildOption {
  pub max_tree_depth: usize,
  pub bin_size: usize,
}

impl Default for TreeBuildOption {
  fn default() -> Self {
    Self {
      max_tree_depth: 15,
      bin_size: 5,
    }
  }
}

impl TreeBuildOption {
  pub fn should_continue(&self, node: &impl SpacePartitionTreeNode, depth: usize) -> bool {
    depth < self.max_tree_depth || node.contained_item_count() > self.bin_size
  }
}

pub struct BuildPrimitive<B: CenterAblePrimitive> {
  pub bounding: B,
  pub center: B::Center,
}

impl<B: CenterAblePrimitive> BuildPrimitive<B> {
  pub fn new(bounding: B) -> Self {
    let center = bounding.get_center();
    Self { bounding, center }
  }
}

pub fn generate_boxes_in_space(count: usize, space_size: f32, box_size: f32) -> Vec<Box3> {
  (0..count)
    .into_iter()
    .map(|_| {
      let center = Vec3::new(random(), random(), random()) * space_size;
      let half_size = Vec3::new(random(), random(), random()) * box_size;
      Box3::new(center - half_size, center + half_size)
    })
    .collect()
}
