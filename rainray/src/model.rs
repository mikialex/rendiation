use crate::material::Material;
use crate::ray::*;

pub struct Model {
    pub geometry: Box<dyn Intersecterable>,
    pub material: Material,
}

impl Model {
    pub fn new(geometry: Box<dyn Intersecterable>, material: Material) -> Self {
        Model { geometry, material }
    }
}
