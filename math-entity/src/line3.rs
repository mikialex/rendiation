use rendiation_math::Mat4;
use rendiation_math::Vec3;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Line3<T = Vec3<f32>> {
  pub start: T,
  pub end: T,
}

impl<T> Line3<T> {
  pub fn new(start: T, end: T) -> Self {
    Self { start, end }
  }

  pub fn iter<'a>(&'a self) -> Line3Iter<'a, T> {
    Line3Iter::new(self)
  }
}

pub struct Line3Iter<'a, T> {
  line3: &'a Line3<T>,
  visit_count: i8,
}

impl<'a, T> Line3Iter<'a, T> {
  pub fn new(line3: &'a Line3<T>) -> Self {
    Self {
      line3,
      visit_count: -1,
    }
  }
}

impl<'a, T: Copy> Iterator for Line3Iter<'a, T> {
  type Item = T;
  fn next(&mut self) -> Option<Self::Item> {
    self.visit_count += 1;
    if self.visit_count == 0 {
      Some(self.line3.start)
    } else if self.visit_count == 1 {
      Some(self.line3.end)
    } else {
      None
    }
  }
}

impl<T: Copy> Line3<T> {
  pub fn swap(&self) -> Self {
    Self::new(self.end, self.start)
  }

  pub fn swap_if(&self, prediction: impl FnOnce(Self) -> bool) -> Self {
    if prediction(*self) {
      self.swap()
    } else {
      *self
    }
  }
}

impl Line3 {
  pub fn length(&self) -> f32 {
    (self.start - self.end).length()
  }
}

use std::ops::Mul;
impl Mul<Mat4<f32>> for Line3 {
  type Output = Self;

  fn mul(self, m: Mat4<f32>) -> Self {
    Self::new(self.start * m, self.end * m)
  }
}
