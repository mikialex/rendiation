use crate::LineSegment;
use rendiation_math::Vector;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Triangle<T, const D: usize> {
  pub a: Vector<T, D>,
  pub b: Vector<T, D>,
  pub c: Vector<T, D>,
}

impl<T, const D: usize> Triangle<T, D> {
  pub fn new(a: T, b: T, c: T) -> Self {
    Self { a, b, c }
  }

  pub fn iter_point<'a>(&'a self) -> Face3Iter<'a, T, D> {
    Face3Iter::new(self)
  }
}

impl<T: Copy, const D: usize> Triangle<T, D> {
  pub fn map<U>(&self, f: impl Fn(T) -> U) -> Triangle<U, D> {
    Triangle {
      a: f(self.a),
      b: f(self.b),
      c: f(self.c),
    }
  }
}

pub struct Face3Iter<'a, T, const D: usize> {
  face3: &'a Triangle<T, D>,
  visit_count: i8,
}

impl<'a, T, const D: usize> Face3Iter<'a, T, D> {
  pub fn new(face3: &'a Triangle<T, D>) -> Self {
    Self {
      face3,
      visit_count: -1,
    }
  }
}

impl<'a, T: Copy, const D: usize> Iterator for Face3Iter<'a, T, D> {
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

impl<T: Copy, const D: usize> Triangle<T, D> {
  pub fn for_each_edge(&self, mut visitor: impl FnMut(LineSegment<T, 3>)) {
    let ab = LineSegment::new(self.a, self.b);
    let bc = LineSegment::new(self.b, self.c);
    let ca = LineSegment::new(self.c, self.a);
    visitor(ab);
    visitor(bc);
    visitor(ca);
  }
}
