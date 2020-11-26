use rendiation_math::Vector;

pub struct HyperSphere<T, const D: usize> {
  pub center: Vector<T, D>,
  pub radius: T,
}

impl<T, const D: usize> HyperSphere<T, D> {
  pub fn new(center: Vector<T, D>, radius: T) -> Self {
    Self { center, radius }
  }
}
