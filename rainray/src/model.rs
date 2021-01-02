use crate::{material::Material, RainRayGeometry};

pub struct Model {
  pub geometry: Box<dyn RainRayGeometry>,
  pub material: Box<dyn Material>,
}

impl Model {
  pub fn new(geometry: impl RainRayGeometry + 'static, material: impl Material + 'static) -> Self {
    Model {
      geometry: Box::new(geometry),
      material: Box::new(material),
    }
  }
}
