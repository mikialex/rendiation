use crate::material::Material;
use crate::ray::*;

pub struct Model {
  pub geometry: Box<dyn RayIntersectAble>,
  pub material: Material,
}

impl Model {
  pub fn new(geometry: impl RayIntersectAble + 'static, material: Material) -> Self {
    Model { geometry: Box::new(geometry), material }
  }
}
