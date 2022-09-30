use std::{iter::FromIterator, ops::Range};

use rendiation_algebra::Vec3;
use rendiation_geometry::Box3;

pub trait CenterAblePrimitive {
  type Center;
  fn get_center(&self) -> Self::Center;
}

impl CenterAblePrimitive for Box3 {
  type Center = Vec3<f32>;

  #[inline(always)]
  fn get_center(&self) -> Vec3<f32> {
    self.center()
  }
}

pub struct TreeBuildOption {
  pub max_tree_depth: usize,
  pub bin_size: usize,
}

impl Default for TreeBuildOption {
  fn default() -> Self {
    Self {
      max_tree_depth: 10,
      bin_size: 50,
    }
  }
}

impl TreeBuildOption {
  pub fn should_continue(&self, item_count: usize, depth: usize) -> bool {
    depth < self.max_tree_depth && item_count > self.bin_size
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

pub fn bounding_from_build_source<B>(
  index_list: &[usize],
  primitives: &[BuildPrimitive<B>],
  range: Range<usize>,
) -> B
where
  B: FromIterator<B> + CenterAblePrimitive + Copy,
{
  index_list
    .get(range)
    .unwrap()
    .iter()
    .map(|index| primitives[*index].bounding)
    .collect()
}

pub fn generate_boxes_in_space(count: usize, space_size: f32, box_size: f32) -> Vec<Box3> {
  use rand::prelude::*;
  use rand_chacha::ChaCha8Rng;

  const SEED: u64 = 0x6246A426A2424AC + 0x1;
  let mut rng = ChaCha8Rng::seed_from_u64(SEED);
  let mut random = || rng.gen::<f32>();

  (0..count)
    .into_iter()
    .map(|_| {
      let center = Vec3::new(random(), random(), random()) * space_size;
      let half_size = Vec3::new(random(), random(), random()) * box_size;
      Box3::new3(center - half_size, center + half_size)
    })
    .collect()
}
