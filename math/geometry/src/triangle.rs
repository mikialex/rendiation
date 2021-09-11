use crate::{LineSegment, Positioned, SpaceEntity};
use rendiation_algebra::{Scalar, SquareMatrixDimension, Vec3};

pub enum FaceSide {
  Front,
  Back,
  Double,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Triangle<V = Vec3<f32>> {
  pub a: V,
  pub b: V,
  pub c: V,
}

impl<T, V, M, U, const D: usize> SpaceEntity<T, D> for Triangle<U>
where
  T: Scalar,
  M: SquareMatrixDimension<D>,
  V: SpaceEntity<T, D, Matrix = M>,
  U: Positioned<Position = V>,
{
  type Matrix = M;
  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self {
    self.a.mut_position().apply_matrix(mat);
    self.b.mut_position().apply_matrix(mat);
    self.c.mut_position().apply_matrix(mat);
    self
  }
}

impl<V> Triangle<V> {
  pub fn new(a: V, b: V, c: V) -> Self {
    Self { a, b, c }
  }

  pub fn iter_point(&self) -> Face3Iter<'_, V> {
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
  pub fn flip(&self) -> Self {
    Triangle {
      a: self.c,
      b: self.b,
      c: self.a,
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
