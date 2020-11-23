#[derive(Debug, Copy, Clone)]
pub struct HyperSphere<T, U> {
  pub center: U,
  pub radius: T,
}

impl<T, U> HyperSphere<T, U> {
  pub fn new(center: U, radius: T) -> Self {
    Self { center, radius }
  }
}
