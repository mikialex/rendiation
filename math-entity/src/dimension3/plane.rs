use crate::{HyperPlane, Triangle};
use rendiation_math::*;

pub type Plane<T = f32> = HyperPlane<T, 3>;

impl<T: Scalar> Plane<T> {
  pub fn distance_to_point(&self, point: Vec3<T>) -> T {
    self.normal.dot(point) + self.constant
  }

  pub fn project_point(&self, point: Vec3<T>) -> Vec3<T> {
    self.normal * (-self.distance_to_point(point)) + point
  }

  pub fn set_components(&mut self, x: T, y: T, z: T, w: T) -> &mut Self {
    let normal = Vec3::new(x, y, z);
    let inverse_normal_length = T::one() / normal.length();
    self.normal = normal.into_normalized();
    self.constant = w * inverse_normal_length;
    self
  }
}

impl<T: Scalar> From<Triangle<Vec3<T>>> for Plane<T> {
  fn from(face: Triangle<Vec3<T>>) -> Plane<T> {
    let v1 = face.b - face.a;
    let v2 = face.c - face.a;
    let normal = v1.cross(v2).into_normalized();
    let constant = normal.dot(face.a);
    Plane::new(normal, constant)
  }
}
