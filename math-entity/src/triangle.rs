use rendiation_math::Vec3;

use crate::LineSegment;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Triangle<T = Vec3<f32>> {
  pub a: T,
  pub b: T,
  pub c: T,
}

impl<T> Triangle<T> {
  pub fn new(a: T, b: T, c: T) -> Self {
    Self { a, b, c }
  }

  pub fn iter_point<'a>(&'a self) -> Face3Iter<'a, T> {
    Face3Iter::new(self)
  }
}

impl<T: Copy> Triangle<T> {
  pub fn map<U>(&self, f: impl Fn(T) -> U) -> Triangle<U> {
    Triangle {
      a: f(self.a),
      b: f(self.b),
      c: f(self.c),
    }
  }
}

pub struct Face3Iter<'a, T> {
  face3: &'a Triangle<T>,
  visit_count: i8,
}

impl<'a, T> Face3Iter<'a, T> {
  pub fn new(face3: &'a Triangle<T>) -> Self {
    Self {
      face3,
      visit_count: -1,
    }
  }
}

impl<'a, T: Copy> Iterator for Face3Iter<'a, T> {
  type Item = T;
  fn next(&mut self) -> Option<Self::Item> {
    self.visit_count += 1;
    if self.visit_count == 0 {
      Some(self.face3.a)
    } else if self.visit_count == 1 {
      Some(self.face3.b)
    } else if self.visit_count == 2 {
      Some(self.face3.c)
    } else {
      None
    }
  }
}

impl<T: Copy> Triangle<T> {
  pub fn for_each_edge(&self, mut visitor: impl FnMut(LineSegment<T>)) {
    let ab = LineSegment::new(self.a, self.b);
    let bc = LineSegment::new(self.b, self.c);
    let ca = LineSegment::new(self.c, self.a);
    visitor(ab);
    visitor(bc);
    visitor(ca);
  }
}
