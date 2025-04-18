use crate::*;

#[derive(Serialize, Deserialize)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Facet)]
pub struct HyperSphere<T, V> {
  pub center: V,
  pub radius: T,
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

impl<T, V, M, const D: usize> SpaceEntity<T, D> for HyperSphere<T, V>
where
  T: Scalar,
  M: SquareMatrixDimension<D> + SquareMatrix<T>,
  V: SpaceEntity<T, D, Matrix = M>,
{
  type Matrix = M;
  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self {
    self.center.apply_matrix(mat);
    self.radius *= mat.max_scale();
    self
  }
}

impl<T, V, M, const D: usize> SolidEntity<T, D> for HyperSphere<T, V>
where
  Self: LebesgueMeasurable<T, D>,
  T: Scalar,
  M: SquareMatrixDimension<D> + SquareMatrix<T>,
  V: SpaceEntity<T, D, Matrix = M> + Copy,
{
  type Center = V;
  fn centroid(&self) -> Self::Center {
    self.center
  }
}

impl<T, V, const D: usize> ContainAble<T, V, D> for HyperSphere<T, V>
where
  Self: SolidEntity<T, D, Center = V>,
  T: Scalar,
  V: SpaceEntity<T, D> + VectorSpace<T> + InnerProductSpace<T>,
{
  fn contains(&self, v: &V) -> bool {
    (*v - self.center).length2() <= self.radius * self.radius
  }
}

impl<T, V, const D: usize> SpaceBounding<T, HyperAABB<V>, D> for HyperSphere<T, V>
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
