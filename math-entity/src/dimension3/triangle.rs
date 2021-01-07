use crate::{Positioned, Triangle};
use rendiation_math::*;

impl<V: Positioned<f32, 3>> Triangle<V> {
  #[inline(always)]
  fn face_normal_unnormalized(&self) -> Vec3<f32> {
    let cb = self.c.position() - self.b.position();
    let ab = self.a.position() - self.b.position();
    cb.cross(ab)
  }
  pub fn face_normal(&self) -> NormalizedVector<f32, Vec3<f32>> {
    self.face_normal_unnormalized().into_normalized()
  }

  pub fn is_front_facing(&self, direction: Vec3<f32>) -> bool {
    self.face_normal_unnormalized().dot(direction) < 0.0
  }
}

impl<V: Positioned<f32, 3>> Triangle<V> {
  /// return None when triangle is degenerated to a point
  pub fn barycentric(&self, p: Vec3<f32>) -> Option<Vec3<f32>> {
    let v0 = self.b.position() - self.a.position();
    let v1 = self.c.position() - self.a.position();
    let v2 = p - self.a.position();

    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);

    let denom = d00 * d11 - d01 * d01;

    if denom == 0.0 {
      return None;
    }

    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;

    Vec3::new(u, v, w).into()
  }
}
