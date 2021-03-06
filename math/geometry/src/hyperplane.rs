use rendiation_algebra::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct HyperPlane<T: Scalar, V> {
  pub normal: NormalizedVector<T, V>,
  pub constant: T,
}

impl<T: Scalar, V> HyperPlane<T, V> {
  pub fn new(normal: NormalizedVector<T, V>, constant: T) -> Self {
    Self { normal, constant }
  }
}
