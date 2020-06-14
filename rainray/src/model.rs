use crate::material::Material;
use crate::ray::*;

pub struct Model {
  pub geometry: Box<dyn RayIntersectAble>,
  pub material: Box<dyn Material>,
}

impl Model {
  pub fn new(geometry: impl RayIntersectAble + 'static, material: impl Material + 'static) -> Self {
    Model {
      geometry: Box::new(geometry),
      material: Box::new(material),
    }
  }
}
