
#[derive(Debug, Copy, Clone)]
pub struct MultiDimensionalLine<T, U> {
  pub normal: U,
  pub constant: T,
}

impl<T, U> MultiDimensionalLine<T, U> {
  pub fn new(normal: U, constant: T) -> Self {
    Self { normal, constant }
  }
}
