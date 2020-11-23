#[derive(Debug, Copy, Clone)]
pub struct HyperPlane<T, U> {
  pub normal: U,
  pub constant: T,
}

impl<T, U> HyperPlane<T, U> {
  pub fn new(normal: U, constant: T) -> Self {
    Self { normal, constant }
  }
}
