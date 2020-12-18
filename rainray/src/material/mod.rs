use crate::frame::*;
use crate::math::*;
use crate::ray::*;
use rendiation_render_entity::color::{Color, LinearRGBColorSpace, RGBColor};

pub mod physical;
pub use physical::*;

pub struct ScatteringEvent {
  pub out_dir: Vec3,
  pub pdf: f32,
}

impl ScatteringEvent {
  pub fn create_next_ray(&self, at_position: Vec3) -> Ray3 {
    Ray3::new(at_position, self.out_dir)
  }
}

pub trait Material: Send + Sync {
  fn scatter(&self, in_dir: Vec3, intersection: &Intersection) -> Option<ScatteringEvent> {
    let (out_dir, cos) = cosine_sample_hemisphere_in_dir(intersection.hit_normal);
    let pdf = cos / PI;
    ScatteringEvent { out_dir, pdf }.into()
  }
  fn bsdf(&self, from_in_dir: Vec3, out_dir: Vec3, intersection: &Intersection) -> Vec3;
}
