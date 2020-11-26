use crate::{Axis3, HyperAABB};
use rendiation_math::*;
use std::iter::FromIterator;

pub type Box3 = HyperAABB<f32, 3>;

impl Default for Box3 {
  fn default() -> Self {
    Self::empty()
  }
}

impl Box3 {
  pub fn new3(min: Vec3<f32>, max: Vec3<f32>) -> Self {
    Self {
      min: min.into(),
      max: max.into(),
    }
  }

  #[inline(always)]
  pub fn new_cube(center: Vec3<f32>, radius: f32) -> Self {
    Self::new_from_center(center, Vec3::splat(radius))
  }

  #[inline(always)]
  pub fn new_from_center(center: Vec3<f32>, half_size: Vec3<f32>) -> Self {
    Self {
      min: (center - half_size).into(),
      max: (center + half_size).into(),
    }
  }

  #[inline(always)]
  pub fn size(&self) -> Vec3<f32> {
    Vec3::new(self.width(), self.height(), self.depth())
  }

  #[inline(always)]
  pub fn half_size(&self) -> Vec3<f32> {
    self.size() * 0.5
  }

  #[inline(always)]
  pub fn width(&self) -> f32 {
    self.max.x - self.min.x
  }

  #[inline(always)]
  pub fn height(&self) -> f32 {
    self.max.y - self.min.y
  }

  #[inline(always)]
  pub fn depth(&self) -> f32 {
    self.max.z - self.min.z
  }

  #[inline(always)]
  pub fn empty() -> Self {
    const INF: f32 = std::f32::INFINITY;
    const N_INF: f32 = std::f32::NEG_INFINITY;
    Self::new(
      Vec3::new(INF, INF, INF).into(),
      Vec3::new(N_INF, N_INF, N_INF).into(),
    )
  }

  #[inline(always)]
  pub fn center(&self) -> Vec3<f32> {
    (self.min + self.max).data * 0.5
  }

  #[rustfmt::skip]
  #[inline(always)]
  pub fn max_corner(&self, direction: Vec3<f32>) -> Vec3<f32> {
    Vec3::new(
      if direction.x > 0. { self.max.x } else { self.min.x },
      if direction.y > 0. { self.max.y } else { self.min.y },
      if direction.z > 0. { self.max.z } else { self.min.z },
    )
  }

  #[inline(always)]
  pub fn longest_axis(&self) -> (Axis3, f32) {
    let x_length = self.max.x - self.min.x;
    let y_length = self.max.y - self.min.y;
    let z_length = self.max.z - self.min.z;

    if x_length > y_length {
      if x_length > z_length {
        (Axis3::X, x_length)
      } else {
        (Axis3::Z, z_length)
      }
    } else {
      if y_length > z_length {
        (Axis3::Y, y_length)
      } else {
        (Axis3::Z, z_length)
      }
    }
  }

  #[inline(always)]
  pub fn expand_by_point(&mut self, point: Vec3<f32>) {
    self.min = self.min.min(point).into();
    self.max = self.max.max(point).into();
  }

  #[inline(always)]
  pub fn union(&mut self, box3: Self) {
    self.expand_by_box(box3)
  }

  #[inline(always)]
  pub fn is_empty(&self) -> bool {
    (self.max.x < self.min.x) || (self.max.y < self.min.y) || (self.max.z < self.min.z)
  }

  #[inline(always)]
  pub fn expand_by_box(&mut self, box3: Self) {
    if self.is_empty() {
      *self = box3;
    }
    self.min = self.min.min(box3.min.data).into();
    self.max = self.max.max(box3.max.data).into();
  }

  pub fn apply_matrix(&self, m: Mat4<f32>) -> Self {
    let points = [
      Vec3::new(self.min.x, self.min.y, self.min.z) * m, // 000
      Vec3::new(self.min.x, self.min.y, self.max.z) * m, // 001
      Vec3::new(self.min.x, self.max.y, self.min.z) * m, // 010
      Vec3::new(self.min.x, self.max.y, self.max.z) * m, // 011
      Vec3::new(self.max.x, self.min.y, self.min.z) * m, // 100
      Vec3::new(self.max.x, self.min.y, self.max.z) * m, // 101
      Vec3::new(self.max.x, self.max.y, self.min.z) * m, // 110
      Vec3::new(self.max.x, self.max.y, self.max.z) * m, // 111
    ];
    points.iter().collect()
  }
}

impl<'a> FromIterator<&'a Vec3<f32>> for Box3 {
  fn from_iter<I: IntoIterator<Item = &'a Vec3<f32>>>(items: I) -> Self {
    let mut bbox = Self::empty();
    items.into_iter().for_each(|p| bbox.expand_by_point(*p));
    bbox
  }
}

impl FromIterator<Vec3<f32>> for Box3 {
  fn from_iter<I: IntoIterator<Item = Vec3<f32>>>(items: I) -> Self {
    let mut bbox = Self::empty();
    items.into_iter().for_each(|p| bbox.expand_by_point(p));
    bbox
  }
}

impl<'a> FromIterator<&'a Box3> for Box3 {
  fn from_iter<I: IntoIterator<Item = &'a Box3>>(items: I) -> Self {
    let mut bbox = Self::empty();
    items.into_iter().for_each(|p| bbox.expand_by_box(*p));
    bbox
  }
}

impl FromIterator<Box3> for Box3 {
  fn from_iter<I: IntoIterator<Item = Box3>>(items: I) -> Self {
    let mut bbox = Self::empty();
    items.into_iter().for_each(|p| bbox.expand_by_box(p));
    bbox
  }
}
