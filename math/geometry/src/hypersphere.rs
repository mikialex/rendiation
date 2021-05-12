use rendiation_algebra::*;

use crate::{
  ContainAble, HyperAABB, InnerProductSpace, SolidEntity, SpaceBounding,
  SpaceEntity,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HyperSphere<T, V> {
  pub center: V,
  pub radius: T,
}

impl<T: Scalar, const D: usize, V> SpaceEntity<T, D> for HyperSphere<T, V> {
  default fn apply_matrix(&mut self, _mat: SquareMatrixType<T, D>) -> &mut Self {
    unimplemented!()
  }
}

impl<T, V> HyperSphere<T, V> {
  pub fn new(center: V, radius: T) -> Self {
    Self { center, radius }
  }
}

impl<T, V> HyperSphere<T, V>
where
  T: Zero,
  V: Zero,
{
  pub fn zero() -> Self {
    Self {
      center: V::zero(),
      radius: T::zero(),
    }
  }
}

impl<T, const D: usize, V> ContainAble<T, V, D> for HyperSphere<T, V>
where
  Self: SolidEntity<T, D, Center = V>,
  T: Scalar,
  V: SpaceEntity<T, D> + VectorSpace<T> + InnerProductSpace<T>,
{
  default fn contains(&self, v: &V) -> bool {
    (*v - self.center).length2() <= self.radius * self.radius
  }
}

impl<T, const D: usize, V> SpaceBounding<T, HyperAABB<V>, D> for HyperSphere<T, V>
where
  Self: SolidEntity<T, D, Center = V>,
  HyperAABB<V>: SolidEntity<T, D, Center = V>,
  V: Vector<T> + VectorSpace<T>,
  T: Scalar,
{
  fn to_bounding(&self) -> HyperAABB<V> {
    HyperAABB {
      min: self.center - Vector::splat(self.radius),
      max: self.center + Vector::splat(self.radius),
    }
  }
}
