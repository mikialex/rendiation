use rendiation_math::Vector;

#[derive(Debug, Copy, Clone)]
pub struct HyperPlane<T, const D: usize> {
  pub normal: Vector<T, D>,
  pub constant: T,
}

impl<T, const D: usize> HyperPlane<T, D> {
  pub fn new(normal: Vector<T, D>, constant: T) -> Self {
    Self { normal, constant }
  }
}
