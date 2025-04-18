use crate::*;

#[repr(C)]
#[derive(Serialize, Deserialize, Facet)]
pub enum FaceSide {
  Front,
  Back,
  Double,
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Facet)]
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

  pub fn iter_point(&self) -> impl Iterator<Item = &V> {
    once(&self.a).chain(once(&self.b)).chain(once(&self.c))
  }
}

impl<V> Triangle<V> {
  pub fn map<U>(self, mut f: impl FnMut(V) -> U) -> Triangle<U> {
    Triangle {
      a: f(self.a),
      b: f(self.b),
      c: f(self.c),
    }
  }
  pub fn filter_map<U>(self, mut f: impl FnMut(V) -> Option<U>) -> Option<Triangle<U>> {
    Triangle {
      a: f(self.a)?,
      b: f(self.b)?,
      c: f(self.c)?,
    }
    .into()
  }

  #[must_use]
  pub fn flip(self) -> Self {
    Triangle {
      a: self.c,
      b: self.b,
      c: self.a,
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

pub type TrianglePointIter<V> = impl Iterator<Item = V>;

impl<U> IntoIterator for Triangle<U> {
  type Item = U;
  type IntoIter = TrianglePointIter<U>;

  #[define_opaque(TrianglePointIter)]
  fn into_iter(self) -> TrianglePointIter<U> {
    once(self.a).chain(once(self.b)).chain(once(self.c))
  }
}

impl<T> From<(T, T, T)> for Triangle<T> {
  fn from(value: (T, T, T)) -> Self {
    Self::new(value.0, value.1, value.2)
  }
}

impl<T> From<Triangle<T>> for (T, T, T) {
  fn from(tri: Triangle<T>) -> Self {
    (tri.a, tri.b, tri.c)
  }
}
