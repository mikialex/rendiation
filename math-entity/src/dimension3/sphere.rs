use crate::{Box3, HyperSphere, LebesgueMeasurable};
use rendiation_math::math::Math;
use rendiation_math::*;

pub type Sphere = HyperSphere<f32, 3>;

impl LebesgueMeasurable<f32, 3> for Sphere {
  #[inline(always)]
  fn measure(&self) -> f32 {
    3.0 / 4.0 * std::f32::consts::PI * self.radius * self.radius * self.radius
  }
}

impl LebesgueMeasurable<f32, 2> for Sphere {
  #[inline(always)]
  fn measure(&self) -> f32 {
    4.0 * std::f32::consts::PI * self.radius * self.radius
  }
}

impl Sphere {
  pub fn new_from_box(box3: Box3) -> Self {
    let center = (box3.max + box3.min) * 0.5;
    let radius = (box3.max - center).length();
    Sphere::new(center, radius)
  }

  // we cant impl from iter trait because it need iter twice
  pub fn from_points<'a, I>(items: &'a I) -> Self
  where
    &'a I: IntoIterator<Item = &'a Vec3<f32>>,
  {
    let box3: Box3 = items.into_iter().collect();
    let center = (box3.max + box3.min) * 0.5;
    let mut max_distance2 = 0.;
    items.into_iter().for_each(|&point| {
      let d = (point - center).length2();
      max_distance2 = max_distance2.max(d);
    });
    Sphere::new(center, max_distance2.sqrt())
  }

  pub fn from_points_and_center<'a, I>(items: &'a I, center: Vec3<f32>) -> Self
  where
    &'a I: IntoIterator<Item = &'a Vec3<f32>>,
  {
    let mut max_distance2 = 0.;
    items.into_iter().for_each(|&point| {
      let d = (point - center).length2();
      max_distance2 = max_distance2.max(d);
    });
    Sphere::new(center, max_distance2.sqrt())
  }

  pub fn apply_matrix(&self, mat: Mat4<f32>) -> Self {
    let mut sphere = *self;
    sphere.center = sphere.center * mat;
    sphere.radius *= mat.max_scale_on_axis();
    sphere
  }
}
