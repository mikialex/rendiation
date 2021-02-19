use rendiation_algebra::Vec2;
use rendiation_geometry::Ray3;

use crate::scene::Scene;

pub mod ao;
pub mod path_trace;
pub use ao::*;
pub use path_trace::*;

pub trait Integrator: Sync {
  fn integrate(&self, scene: &Scene, ray: Ray3) -> Color<f32, LinearRGBColorSpace<f32>>;
}
