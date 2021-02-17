use rendiation_algebra::{Scalar, SpaceEntity, SquareMatrixType, VectorImpl, VectorType};

use crate::SpaceLineSegment;

pub struct QuadraticBezierSegment<T: Scalar, const D: usize> {
  pub from: VectorType<T, D>,
  pub ctrl: VectorType<T, D>,
  pub to: VectorType<T, D>,
}

impl<T: Scalar, const D: usize> SpaceEntity<T, D> for QuadraticBezierSegment<T, D> {
  fn apply_matrix(&mut self, mat: SquareMatrixType<T, D>) -> &mut Self {
    self.from.apply_matrix(mat);
    self.ctrl.apply_matrix(mat);
    self.to.apply_matrix(mat);
    self
  }
}

impl<T, const D: usize> SpaceLineSegment<T, D> for QuadraticBezierSegment<T, D>
where
  T: Scalar,
  VectorType<T, D>: VectorImpl,
{
  fn start(&self) -> VectorType<T, D> {
    self.from
  }
  fn end(&self) -> VectorType<T, D> {
    self.to
  }
  fn sample(&self, t: T) -> VectorType<T, D> {
    let t2 = t * t;
    let one_t = T::one() - t;
    let one_t2 = one_t * one_t;

    self.from * one_t2 + self.ctrl * T::two() * one_t * t + self.to * t2
  }
}

pub struct CubicBezierSegment<T: Scalar, const D: usize> {
  pub from: VectorType<T, D>,
  pub ctrl1: VectorType<T, D>,
  pub ctrl2: VectorType<T, D>,
  pub to: VectorType<T, D>,
}

impl<T: Scalar, const D: usize> SpaceEntity<T, D> for CubicBezierSegment<T, D> {
  fn apply_matrix(&mut self, mat: SquareMatrixType<T, D>) -> &mut Self {
    self.from.apply_matrix(mat);
    self.ctrl1.apply_matrix(mat);
    self.ctrl2.apply_matrix(mat);
    self.to.apply_matrix(mat);
    self
  }
}

impl<T, const D: usize> SpaceLineSegment<T, D> for CubicBezierSegment<T, D>
where
  T: Scalar,
  VectorType<T, D>: VectorImpl,
{
  fn start(&self) -> VectorType<T, D> {
    self.from
  }
  fn end(&self) -> VectorType<T, D> {
    self.to
  }
  fn sample(&self, t: T) -> VectorType<T, D> {
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
