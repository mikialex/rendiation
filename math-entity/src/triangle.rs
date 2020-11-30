use rendiation_math::{Scalar, SquareMatrixType, Vec3};

use crate::{LineSegment, Positioned, SpaceEntity};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Triangle<V = Vec3<f32>> {
  pub a: V,
  pub b: V,
  pub c: V,
}

impl<T: Scalar, V: Positioned<T, D>, const D: usize> SpaceEntity<T, D> for Triangle<V> {
  fn apply_matrix(&mut self, mat: &SquareMatrixType<T, D>) {
    self.a.position_mut().apply_matrix(mat);
    self.b.position_mut().apply_matrix(mat);
    self.c.position_mut().apply_matrix(mat);
  }
}

impl<V> Triangle<V> {
  pub fn new(a: V, b: V, c: V) -> Self {
    Self { a, b, c }
  }

  pub fn iter_point<'a>(&'a self) -> Face3Iter<'a, V> {
    Face3Iter::new(self)
  }
}

impl<V: Copy> Triangle<V> {
  pub fn map<U>(&self, f: impl Fn(V) -> U) -> Triangle<U> {
    Triangle {
      a: f(self.a),
      b: f(self.b),
      c: f(self.c),
    }
  }
}

pub struct Face3Iter<'a, V> {
  face3: &'a Triangle<V>,
  visit_count: i8,
}

impl<'a, V> Face3Iter<'a, V> {
  pub fn new(face3: &'a Triangle<V>) -> Self {
    Self {
      face3,
      visit_count: -1,
    }
  }
}

impl<'a, V: Copy> Iterator for Face3Iter<'a, V> {
  type Item = V;
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

impl<V: Copy> Triangle<V> {
  pub fn for_each_edge(&self, mut visitor: impl FnMut(LineSegment<V>)) {
    let ab = LineSegment::new(self.a, self.b);
    let bc = LineSegment::new(self.b, self.c);
    let ca = LineSegment::new(self.c, self.a);
    visitor(ab);
    visitor(bc);
    visitor(ca);
  }
}
