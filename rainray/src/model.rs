use crate::material::Material;
use crate::ray::*;

pub struct Model {
  pub geometry: Box<dyn RayIntersectAble>,
  pub material: Material,
}

impl Model {
  pub fn new(geometry: Box<dyn RayIntersectAble>, material: Material) -> Self {
    Model { geometry, material }
  }
}
