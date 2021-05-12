#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HyperAABB<V> {
  pub min: V,
  pub max: V,
}

impl<V> HyperAABB<V> {
  pub fn new(min: V, max: V) -> Self {
    Self { min, max }
  }
}

// impl<T: Scalar> SolidEntity<T, 3> for Box3<T> {
//   type Center = Vec3<T>;
//   fn centroid(&self) -> Vec3<T> {
//     (self.min + self.max) * T::half()
//   }
// }
