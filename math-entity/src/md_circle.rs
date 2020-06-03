
#[derive(Debug, Copy, Clone)]
pub struct MultiDimensionalCircle<T, U> {
  pub center: U,
  pub radius: T,
}

impl<T, U> MultiDimensionalCircle<T, U> {
  pub fn new(center: U, radius: T) -> Self {
    Self { center, radius }
  }
}
