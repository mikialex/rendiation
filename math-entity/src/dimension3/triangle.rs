use crate::Triangle;
use rendiation_math::*;

pub type Triangle3D<T = f32> = Triangle<Vec3<T>>;

impl<T: Scalar> Triangle3D<T> {
  #[inline(always)]
  fn face_normal_unnormalized(&self) -> Vec3<T> {
    let cb = self.c - self.b;
    let ab = self.a - self.b;
    cb.cross(ab)
  }
  pub fn face_normal(&self) -> NormalizedVector<T, Vec3<T>> {
    self.face_normal_unnormalized().into_normalized()
  }

  pub fn is_front_facing(&self, direction: Vec3<T>) -> bool {
    self.face_normal_unnormalized().dot(direction) < T::zero()
  }
}

impl<T: Scalar> Triangle3D<T> {
  /// return None when triangle is degenerated to a point
  pub fn barycentric(&self, p: Vec3<T>) -> Option<Vec3<T>> {
    let v0 = self.b - self.a;
    let v1 = self.c - self.a;
    let v2 = p - self.a;

    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);

    let denom = d00 * d11 - d01 * d01;

    if denom == T::zero() {
      return None;
    }

    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = T::one() - v - w;

    Vec3::new(u, v, w).into()
  }
}
