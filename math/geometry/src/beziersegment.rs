use rendiation_algebra::{Scalar, SpaceEntity, SquareMatrix, SquareMatrixDimension, VectorSpace};

use crate::SpaceLineSegment;

pub struct QuadraticBezierSegment<V> {
  pub from: V,
  pub ctrl: V,
  pub to: V,
}

impl<T, M, V, const D: usize> SpaceEntity<T, D> for QuadraticBezierSegment<V>
where
  M: SquareMatrixDimension<D> + SquareMatrix<T>,
  V: SpaceEntity<T, D, Matrix = M> + Copy,
  T: Scalar,
{
  type Matrix = M;
  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self {
    self.from.apply_matrix(mat);
    self.ctrl.apply_matrix(mat);
    self.to.apply_matrix(mat);
    self
  }
}

impl<T, V> SpaceLineSegment<T, V> for QuadraticBezierSegment<V>
where
  T: Scalar,
  V: VectorSpace<T>,
{
  fn start(&self) -> V {
    self.from
  }
  fn end(&self) -> V {
    self.to
  }
  fn sample(&self, t: T) -> V {
    let t2 = t * t;
    let one_t = T::one() - t;
    let one_t2 = one_t * one_t;

    self.from * one_t2 + self.ctrl * T::two() * one_t * t + self.to * t2
  }
}

pub struct CubicBezierSegment<V> {
  pub from: V,
  pub ctrl1: V,
  pub ctrl2: V,
  pub to: V,
}

impl<T, M, V, const D: usize> SpaceEntity<T, D> for CubicBezierSegment<V>
where
  M: SquareMatrixDimension<D> + SquareMatrix<T>,
  V: SpaceEntity<T, D, Matrix = M> + Copy,
  T: Scalar,
{
  type Matrix = M;
  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self {
    self.from.apply_matrix(mat);
    self.ctrl1.apply_matrix(mat);
    self.ctrl2.apply_matrix(mat);
    self.to.apply_matrix(mat);
    self
  }
}

impl<T, V> SpaceLineSegment<T, V> for CubicBezierSegment<V>
where
  T: Scalar,
  V: VectorSpace<T>,
{
  fn start(&self) -> V {
    self.from
  }
  fn end(&self) -> V {
    self.to
  }
  fn sample(&self, t: T) -> V {
    let t2 = t * t;
    let t3 = t2 * t;
    let one_t = T::one() - t;
    let one_t2 = one_t * one_t;
    let one_t3 = one_t2 * one_t;

    self.from * one_t3
      + self.ctrl1 * T::three() * one_t2 * t
      + self.ctrl2 * T::three() * one_t * t2
      + self.to * t3
  }
}
