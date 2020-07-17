use crate::environment::*;
use crate::light::*;
use crate::model::*;
use crate::ray::*;
use rendiation_math_entity::Ray3;
use std::sync::Arc;

pub struct Scene {
  pub models: Vec<Arc<Model>>,
  pub point_lights: Vec<PointLight>,
  pub env: Box<dyn Environment>,
}

impl Scene {
  pub fn get_min_dist_hit(&self, ray: &Ray3) -> Option<(Intersection, Arc<Model>)> {
    let mut min_distance = std::f32::INFINITY;
    let mut result: Option<(Intersection, Arc<Model>)> = None;
    for model in &self.models {
      if let Some(intersection) = model.geometry.intersect(ray) {
        if intersection.distance < min_distance {
          min_distance = intersection.distance;
          result = Some((intersection, model.clone()))
        }
      }
    }
    result
  }
}
