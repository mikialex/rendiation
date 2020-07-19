use crate::{Box3, MultiDimensionalCircle};
use rendiation_math::math::Math;
use rendiation_math::*;

pub type Sphere = MultiDimensionalCircle<f32, Vec3<f32>>;

impl Sphere {
  pub fn new_from_box(box3: Box3) -> Self {
    let center = (box3.max + box3.min) / 2.;
    let radius = (box3.max - center).length();
    Sphere::new(center, radius)
  }

  pub fn from_points<'a, I>(items: &'a I) -> Self
  where
    &'a I: IntoIterator<Item = &'a Vec3<f32>>,
  {
    let box3 = Box3::from_points(items.into_iter());
    let center = (box3.max + box3.min) / 2.;
    let mut max_distance2 = 0.;
    items.into_iter().for_each(|&point| {
      let d = (point - center).length2();
      max_distance2 = max_distance2.max(d);
    });
    Sphere::new(center, max_distance2.sqrt())
  }

  pub fn apply_matrix(mut self, mat: Mat4<f32>) -> Self {
    self.center = self.center * mat;
    self.radius *= mat.max_scale_on_axis();
    self
  }
}
