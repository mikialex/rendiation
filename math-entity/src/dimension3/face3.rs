use crate::{PositionedPoint3, Line3};
use rendiation_math::Vec3;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Face3<T = Vec3<f32>> {
  pub a: T,
  pub b: T,
  pub c: T,
}

impl<T> Face3<T> {
  pub fn new(a: T, b: T, c: T) -> Self {
    Self { a, b, c }
  }

  pub fn iter_point<'a>(&'a self) -> Face3Iter<'a, T> {
    Face3Iter::new(self)
  }
}

impl<T: PositionedPoint3> Face3<T> {
  pub fn face_normal_by_position(&self) -> Vec3<f32> {
    let cb = self.a.position() - self.b.position();
    let ab = self.a.position() - self.b.position();
    let n = cb.cross(ab);
    n.normalize()
  }
}

pub struct Face3Iter<'a, T> {
  face3: &'a Face3<T>,
  visit_count: i8,
}

impl<'a, T> Face3Iter<'a, T> {
  pub fn new(face3: &'a Face3<T>) -> Self {
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

impl<T: Copy> Face3<T> {
  pub fn for_each_edge(&self, mut visitor: impl FnMut(Line3<T>)) {
    let ab = Line3::new(self.a, self.b);
    let bc = Line3::new(self.b, self.c);
    let ca = Line3::new(self.c, self.a);
    visitor(ab);
    visitor(bc);
    visitor(ca);
  }
}

impl Face3 {
  /// return null when point is outside of triangle
  pub fn barycentric(&self, p: Vec3<f32>) -> Option<Vec3<f32>> {
    let v0 = self.b - self.a;
    let v1 = self.c - self.a;
    let v2 = p - self.a;

    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);

    let denom = d00 * d11 - d01 * d01;

    if denom == 0.0 {
      return None;
    }

    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;

    Some(Vec3::new(u, v, w))
  }
}
