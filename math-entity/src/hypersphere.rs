use rendiation_math::{DimensionalVec, VectorMark};

#[derive(Debug, Copy, Clone)]
pub struct HyperSphere<T, const D: usize> {
  pub center: <VectorMark<T> as DimensionalVec<T, D>>::Type,
  pub radius: T,
}

impl<T, const D: usize> HyperSphere<T, D> {
  pub fn new(center: <VectorMark<T> as DimensionalVec<T, D>>::Type, radius: T) -> Self {
    Self { center, radius }
  }
}
