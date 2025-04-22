use rendiation_algebra::*;

use crate::{DistanceTo, HyperPlane, Triangle};

pub type Plane<T = f32> = HyperPlane<T, Vec3<T>>;

impl<T: Scalar> DistanceTo<Vec3<T>, T> for Plane<T> {
  fn distance_to(&self, point: &Vec3<T>) -> T {
    self.normal.dot(*point) + self.constant
  }
}

impl<T: Scalar> SpaceEntity<T, 3> for Plane<T> {
  type Matrix = Mat4<T>;

  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self {
    let v = Vec4::new(self.normal.x, self.normal.y, self.normal.z, self.constant);
    let v = mat.inverse_or_identity().transpose() * v;
    self.set_components(v.x, v.y, v.z, v.w);
    self
  }
}

impl<T: Scalar> Plane<T> {
  pub fn project_point(&self, point: Vec3<T>) -> Vec3<T> {
    self.normal * (-self.distance_to(&point)) + point
  }

  pub fn from_components(x: T, y: T, z: T, w: T) -> Self {
    let normal = Vec3::new(x, y, z).into_normalized();
    let inverse_normal_length = T::one() / normal.length();
    Self {
      normal,
      constant: w * inverse_normal_length,
    }
  }

  pub fn set_components(&mut self, x: T, y: T, z: T, w: T) -> &mut Self {
    *self = Self::from_components(x, y, z, w);
    self
  }
}

impl<T: Scalar> From<Triangle<Vec3<T>>> for Plane<T> {
  fn from(face: Triangle<Vec3<T>>) -> Plane<T> {
    let v1 = face.b - face.a;
    let v2 = face.c - face.a;
    Self::from_normal_and_plane_point(v1.cross(v2), face.a)
  }
}
