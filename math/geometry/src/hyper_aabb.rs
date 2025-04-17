use crate::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Facet)]
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

  #[inline(always)]
  pub fn expand_by_other<T>(&mut self, other: Self)
  where
    T: Scalar,
    V: RealVector<T>,
  {
    self.min = self.min.min(other.min);
    self.max = self.max.max(other.max);
  }

  #[inline(always)]
  pub fn union<T>(&mut self, other: Self)
  where
    T: Scalar,
    V: RealVector<T>,
  {
    self.expand_by_other(other)
  }

  #[inline(always)]
  pub fn union_into<T>(mut self, other: Self) -> Self
  where
    T: Scalar,
    V: RealVector<T>,
  {
    self.expand_by_other(other);
    self
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

impl<T, V, const D: usize> SpaceBounding<T, HyperSphere<T, V>, D> for HyperAABB<V>
where
  T: Scalar,
  HyperSphere<T, V>: SolidEntity<T, D>,
  Self: SolidEntity<T, D>,
  V: InnerProductSpace<T>,
{
  #[inline(always)]
  fn to_bounding(&self) -> HyperSphere<T, V> {
    let center = (self.max + self.min) * T::half();
    let radius = (self.max - center).length();
    HyperSphere::new(center, radius)
  }
}
