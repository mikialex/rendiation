#[derive(Debug, Copy, Clone)]
pub struct AABB<T> {
  pub min: T,
  pub max: T,
}

impl<T> AABB<T> {
  pub fn new(min: T, max: T) -> Self {
    Self { min, max }
  }
}
