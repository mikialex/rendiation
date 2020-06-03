#[derive(Debug, Copy, Clone)]
pub struct Ray<T> {
  pub origin: T,
  pub direction: T,
}

impl<T> Ray<T> {
  pub fn new(origin: T, direction: T) -> Self {
    Ray { origin, direction }
  }
}
