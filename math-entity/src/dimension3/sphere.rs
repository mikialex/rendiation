use crate::{Box3, HyperSphere};
use rendiation_math::math::Math;
use rendiation_math::*;

pub type Sphere = HyperSphere<f32, 3>;

impl Sphere {
  pub fn zero() -> Self {
    Sphere::new(Vec3::new(0.0, 0.0, 0.0), 0.0)
  }

  pub fn new_from_box(box3: Box3) -> Self {
    let center = (box3.max + box3.min) / 2.;
    let radius = (box3.max - center).length();
    Sphere::new(center, radius)
  }

  // we cant impl from iter trait because it need iter twice
  pub fn from_points<'a, I>(items: &'a I) -> Self
  where
    &'a I: IntoIterator<Item = &'a Vec3<f32>>,
  {
    let box3: Box3 = items.into_iter().collect();
    let center = (box3.max + box3.min) / 2.;
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
    sphere.center = self.center * mat;
    sphere.radius *= mat.max_scale_on_axis();
    sphere
  }
}
