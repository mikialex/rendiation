use rendiation_math::*;
use crate::Face3;

#[derive(Debug, Copy, Clone)]
pub struct Plane {
  pub normal: Vec3<f32>,
  pub constant: f32,
}

impl Plane {
  pub fn new(normal: Vec3<f32>, constant: f32) -> Self {
    Plane { normal, constant }
  }

  pub fn distance_to_point(&self, point: Vec3<f32>) -> f32 {
    self.normal.dot(point) + self.constant
  }

  pub fn project_point(&self,point: Vec3<f32>) -> Vec3<f32> {
    self.normal * (- self.distance_to_point(point)) + point
  }

  pub fn set_components(&mut self, x: f32, y: f32, z: f32, w: f32) -> &mut Self {
    self.normal.set(x, y, z);
    self.constant = w;
    self
  }

  pub fn normalize(&mut self) -> &mut Self {
    let inverse_normal_length = 1.0 / self.normal.length();
    self.normal *= inverse_normal_length;
    self.constant *= inverse_normal_length;
    self
  }
}

impl From<Face3> for Plane {
  fn from(face: Face3) -> Plane {
    let v1 = face.b - face.a;
    let v2 = face.c - face.a;
    let normal = v1.cross(v2).normalize();
    let constant = normal.dot(face.a);
    Plane::new(normal, constant)
  }
}