use rendiation_algebra::{Scalar, SpaceEntity, VectorSpace};

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

impl<T, const D: usize, V> SolidEntity<T, D> for HyperAABB<V>
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
