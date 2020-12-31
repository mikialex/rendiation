use rendiation_math::Vec2;
use rendiation_math_entity::Ray3;
use rendiation_render_entity::{
  color::{Color, LinearRGBColorSpace},
  Camera,
};

use crate::scene::Scene;

pub mod ao;
pub mod path_trace;
pub use ao::*;
pub use path_trace::*;

pub trait Integrator: Sync {
  fn integrate(&self, scene: &Scene, ray: Ray3) -> Color<f32, LinearRGBColorSpace<f32>>;
}
