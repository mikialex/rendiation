use rendiation_math::math::Math;
use rendiation_math::*;

#[derive(Debug, Copy, Clone)]
pub enum Axis {
  X,
  Y,
  Z,
}

#[derive(Debug, Copy, Clone)]
pub struct Box3 {
  pub min: Vec3<f32>,
  pub max: Vec3<f32>,
}

impl Box3 {
  pub fn new(min: Vec3<f32>, max: Vec3<f32>) -> Self {
    Box3 { min, max }
  }

  pub fn from_3points(p1: Vec3<f32>, p2: Vec3<f32>, p3: Vec3<f32>) -> Box3 {
    Box3 {
      min: p1.min(p2).min(p3),
      max: p1.max(p2).max(p3),
    }
  }

  pub fn empty() -> Self {
    const INF: f32 = std::f32::INFINITY;
    const N_INF: f32 = std::f32::NEG_INFINITY;
    Box3::new(Vec3::new(INF, INF, INF), Vec3::new(N_INF, N_INF, N_INF))
  }

  pub fn center(&self) -> Vec3<f32> {
    (self.min + self.max) * 0.5
  }

  pub fn longest_axis(&self) -> Axis {
    let x_length = self.max.x - self.min.x;
    let y_length = self.max.y - self.min.y;
    let z_length = self.max.z - self.min.z;

    if x_length > y_length {
      if x_length > z_length {
        Axis::X
      } else {
        Axis::Z
      }
    } else {
      if y_length > z_length {
        Axis::Y
      } else {
        Axis::Z
      }
    }
  }

  pub fn expand_by_point(&mut self, point: Vec3<f32>) {
    self.min.min(point);
    self.max.max(point);
  }

  pub fn expand_by_box(&mut self, box3: Box3) {
    self.min.min(box3.min);
    self.max.max(box3.max);
  }

  pub fn from_points(iter: impl Iterator<Item = Vec3<f32>>) -> Self {
    let mut bbox = Box3::empty();
    iter.for_each(|p| bbox.expand_by_point(p));
    bbox
  }

  pub fn from_boxes(iter: impl Iterator<Item = Box3>) -> Self {
    let mut bbox = Box3::empty();
    iter.for_each(|p| bbox.expand_by_box(p));
    bbox
  }
}

use std::ops::Mul;
impl Mul<Mat4<f32>> for Box3 {
  type Output = Self;

  fn mul(self, m: Mat4<f32>) -> Self {
    Self::new(self.min * m, self.max * m)
  }
}
