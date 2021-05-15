use std::iter::FromIterator;

use crate::{ContainAble, HyperAABB, LebesgueMeasurable};
use rendiation_algebra::{Mat3, Scalar, SpaceEntity, Vec2};

pub type Rectangle<T = f32> = HyperAABB<Vec2<T>>;

impl<T: Scalar> Rectangle<T> {
  pub fn width(&self) -> T {
    self.max.x - self.min.x
  }

  pub fn height(&self) -> T {
    self.max.y - self.min.y
  }

  #[inline(always)]
  pub fn is_empty(&self) -> bool {
    (self.max.x < self.min.x) || (self.max.y < self.min.y)
  }
}

impl<T: Scalar> LebesgueMeasurable<T, 2> for Rectangle<T> {
  #[inline(always)]
  fn measure(&self) -> T {
    self.width() * self.height()
  }
}

impl<T: Scalar> SpaceEntity<T, 2> for Rectangle<T> {
  type Matrix = Mat3<T>;
  #[inline(always)]
  fn apply_matrix(&mut self, m: Self::Matrix) -> &mut Self {
    if self.is_empty() {
      return self;
    }
    let points = [
      *Vec2::new(self.min.x, self.min.y).apply_matrix(m), // 00
      *Vec2::new(self.min.x, self.min.y).apply_matrix(m), // 01
      *Vec2::new(self.min.x, self.max.y).apply_matrix(m), // 10
      *Vec2::new(self.min.x, self.max.y).apply_matrix(m), // 11
    ];
    *self = points.iter().collect();
    self
  }
}

impl<T: Scalar> ContainAble<T, Vec2<T>, 2> for Rectangle<T> {
  fn contains(&self, v: &Vec2<T>) -> bool {
    v.x >= self.min.x && v.x <= self.max.x && v.y >= self.min.y && v.y <= self.max.y
  }
}

impl<T: Scalar> FromIterator<Vec2<T>> for Rectangle<T> {
  fn from_iter<I: IntoIterator<Item = Vec2<T>>>(items: I) -> Self {
    let mut bbox = Self::empty();
    items.into_iter().for_each(|p| bbox.expand_by_point(p));
    bbox
  }
}

impl<'a, T: Scalar> FromIterator<&'a Vec2<T>> for Rectangle<T> {
  fn from_iter<I: IntoIterator<Item = &'a Vec2<T>>>(items: I) -> Self {
    let mut bbox = Self::empty();
    items.into_iter().for_each(|p| bbox.expand_by_point(*p));
    bbox
  }
}
