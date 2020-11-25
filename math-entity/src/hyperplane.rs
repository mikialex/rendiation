use rendiation_math::{DimensionalVec, VectorMark};

#[derive(Debug, Copy, Clone)]
pub struct HyperPlane<T, const D: usize> {
  pub normal: <VectorMark<T> as DimensionalVec<T, D>>::Type,
  pub constant: T,
}

impl<T, const D: usize> HyperPlane<T, D> {
  pub fn new(normal: <VectorMark<T> as DimensionalVec<T, D>>::Type, constant: T) -> Self {
    Self { normal, constant }
  }
}
