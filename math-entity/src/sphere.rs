use crate::box3::Box3;
use rendiation_math::math::Math;
use rendiation_math::*;

#[derive(Debug, Copy, Clone)]
pub struct Sphere {
  pub center: Vec3<f32>,
  pub radius: f32,
}

impl Sphere {
  pub fn new(center: Vec3<f32>, radius: f32) -> Self {
    Sphere { center, radius }
  }

  pub fn new_from_box(box3: Box3) -> Self {
    let center = (box3.max + box3.min) / 2.;
    let radius = (box3.max - center).length();
    Sphere::new(center, radius)
  }

  pub fn make_from_position_buffer_with_box(position: &[f32], box3: &Box3) -> Self {
    let center = (box3.max + box3.min) / 2.;
    let mut max_distance2 = 0.;
    for index in 0..position.len() / 3 {
      let i = index * 3;
      let p = Vec3::new(position[i], position[i + 1], position[i + 2]);
      let d = (p - center).length2();
      max_distance2 = max_distance2.max(d);
    }
    Sphere::new(center, max_distance2.sqrt())
  }

  // iter reuse issue
  // pub fn from_points(iter: &mut impl Iterator<Item = Vec3<f32>>) -> Self {
  //   let box3 = Box3::from_points(iter);
  //   let center = (box3.max + box3.min) / 2.;
  //   let mut max_distance2 = 0.;
  //   iter.for_each(|point|{
  //     let d = (point - center).length2();
  //     max_distance2 = max_distance2.max(d);
  //   });
  //   Sphere::new(center, max_distance2.sqrt())
  // }

  pub fn apply_matrix(mut self, mat: &Mat4<f32>) -> Self {
    self.center = self.center.apply_mat4(mat);
    self.radius *= mat.max_scale_on_axis();
    self
  }
}
