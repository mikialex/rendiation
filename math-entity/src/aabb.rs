use rendiation_math::Vec3;

#[derive(Debug, Copy, Clone)]
pub struct AABB<T = Vec3<f32>> {
  pub min: T,
  pub max: T,
}

impl<T> AABB<T> {
  pub fn new(min: T, max: T) -> Self {
    Self { min, max }
  }
}