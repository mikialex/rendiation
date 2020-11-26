use rendiation_math::*;

pub struct HyperAABB<T, const D: usize> {
  pub min: Vector<T, D>,
  pub max: Vector<T, D>,
}

impl<T, const D: usize> Copy for HyperAABB<T, D>
where
  T: Copy,
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Copy,
{
}

impl<T, const D: usize> Clone for HyperAABB<T, D>
where
  T: Clone,
  <VectorMark<T> as DimensionalVec<T, D>>::Type: Clone,
{
  fn clone(&self) -> Self {
    Self {
      min: self.min.clone(),
      max: self.max.clone(),
    }
  }
}

impl<T, const D: usize> HyperAABB<T, D> {
  pub fn new(min: Vector<T, D>, max: Vector<T, D>) -> Self {
    Self { min, max }
  }
}
