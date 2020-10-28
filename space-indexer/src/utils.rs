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
