use crate::*;

pub type QuadraticBezierSegment<V> = SpaceLineSegment<V, QuadraticBezierShape<V>>;
pub type QuadraticBezierSegment2D<T> = QuadraticBezierSegment<Vec2<T>>;

pub struct QuadraticBezierShape<V> {
  pub ctrl: V,
}

impl<T, M, V, const D: usize> SpaceEntity<T, D> for QuadraticBezierShape<V>
where
  M: SquareMatrixDimension<D> + SquareMatrix<T>,
  V: SpaceEntity<T, D, Matrix = M> + Copy,
  T: Scalar,
{
  type Matrix = M;
  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self {
    self.ctrl.apply_matrix(mat);
    self
  }
}

impl<T, V> SpaceLineSegmentShape<T, V> for QuadraticBezierShape<V>
where
  T: Scalar,
  V: VectorSpace<T>,
{
  fn sample(&self, t: T, start: &V, end: &V) -> V {
    let t2 = t * t;
    let one_t = T::one() - t;
    let one_t2 = one_t * one_t;

    *start * one_t2 + self.ctrl * T::two() * one_t * t + *end * t2
  }
}

pub type CubicBezierSegment<U> = SpaceLineSegment<U, CubicBezierShape<U>>;
pub type CubicBezierSegment2D<T> = CubicBezierSegment<Vec2<T>>;

pub struct CubicBezierShape<V> {
  pub ctrl1: V,
  pub ctrl2: V,
}

impl<T, M, V, const D: usize> SpaceEntity<T, D> for CubicBezierShape<V>
where
  M: SquareMatrixDimension<D> + SquareMatrix<T>,
  V: SpaceEntity<T, D, Matrix = M> + Copy,
  T: Scalar,
{
  type Matrix = M;
  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self {
    self.ctrl1.apply_matrix(mat);
    self.ctrl2.apply_matrix(mat);
    self
  }
}

impl<T, V> SpaceLineSegmentShape<T, V> for CubicBezierShape<V>
where
  T: Scalar,
  V: VectorSpace<T>,
{
  fn sample(&self, t: T, start: &V, end: &V) -> V {
    let t2 = t * t;
    let t3 = t2 * t;
    let one_t = T::one() - t;
    let one_t2 = one_t * one_t;
    let one_t3 = one_t2 * one_t;

    *start * one_t3
      + self.ctrl1 * T::three() * one_t2 * t
      + self.ctrl2 * T::three() * one_t * t2
      + *end * t3
  }
}
