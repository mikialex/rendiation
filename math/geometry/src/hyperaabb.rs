use rendiation_algebra::{RealVector, Scalar, SpaceEntity, Vector, VectorSpace};

use crate::{LebesgueMeasurable, SolidEntity};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HyperAABB<V> {
  pub min: V,
  pub max: V,
}

impl<V> HyperAABB<V> {
  pub fn new(min: V, max: V) -> Self {
    Self { min, max }
  }
}

impl<V> HyperAABB<V> {
  #[inline(always)]
  pub fn empty<T>() -> Self
  where
    T: Scalar,
    V: Vector<T>,
  {
    Self::new(
      Vector::splat(T::infinity()),
      Vector::splat(T::neg_infinity()),
    )
  }

  #[inline(always)]
  pub fn expand_by_point<T>(&mut self, point: V)
  where
    T: Scalar,
    V: RealVector<T>,
  {
    self.min = self.min.min(point);
    self.max = self.max.max(point);
  }
}

impl<T, V, const D: usize> SolidEntity<T, D> for HyperAABB<V>
where
  T: Scalar,
  Self: LebesgueMeasurable<T, D>,
  Self: SpaceEntity<T, D>,
  V: VectorSpace<T>,
{
  type Center = V;
  fn centroid(&self) -> V {
    (self.min + self.max) * T::half()
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HyperAABBBySize<V> {
  pub position: V,
  pub size: V,
}

impl<V> HyperAABBBySize<V> {
  pub fn new(position: V, size: V) -> Self {
    Self { position, size }
  }
}
