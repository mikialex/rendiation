use rendiation_math::*;

use crate::{ContainAble, LebesgueMeasurable, SolidEntity, SpaceEntity};

pub struct HyperSphere<T: Scalar, const D: usize> {
  pub center: VectorType<T, D>,
  pub radius: T,
}

impl<T: Scalar, const D: usize> SpaceEntity<T, D> for HyperSphere<T, D> {
  default fn apply_matrix(&mut self, _mat: &SquareMatrixType<T, D>) -> &mut Self {
    unimplemented!()
  }
}

/// https://en.wikipedia.org/wiki/Volume_of_an_n-ball
impl<T: Scalar, const D: usize> LebesgueMeasurable<T, D> for HyperSphere<T, D> {
  default fn measure(&self) -> T {
    unimplemented!()
  }
}

impl<T: Scalar, const D: usize> SolidEntity<T, D> for HyperSphere<T, D> {}

impl<T: Scalar, const D: usize> Copy for HyperSphere<T, D> where VectorType<T, D>: Copy {}

impl<T: Scalar, const D: usize> Clone for HyperSphere<T, D>
where
  VectorType<T, D>: Clone,
{
  fn clone(&self) -> Self {
    Self {
      center: self.center,
      radius: self.radius,
    }
  }
}

impl<T: Scalar, const D: usize> HyperSphere<T, D> {
  pub fn new(center: VectorType<T, D>, radius: T) -> Self {
    Self { center, radius }
  }
}

impl<T, const D: usize> HyperSphere<T, D>
where
  T: Scalar,
  VectorType<T, D>: Zero,
{
  pub fn zero() -> Self {
    Self {
      center: <VectorMark<T> as DimensionalVec<T, D>>::Type::zero(),
      radius: T::zero(),
    }
  }
}

impl<T: Scalar, const D: usize> ContainAble<T, VectorType<T, D>, D> for HyperSphere<T, D> {
  default fn contains(&self, v: &VectorType<T, D>) -> bool {
    (*v - self.center).length2() <= self.radius * self.radius
  }
}
