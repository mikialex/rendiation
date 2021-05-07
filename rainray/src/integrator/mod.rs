use rendiation_color::{Color, LinearRGBColorSpace};
use rendiation_geometry::Ray3;

use crate::RayTraceScene;

pub mod ao;
pub mod path_trace;
pub use ao::*;
pub use path_trace::*;

pub trait Integrator: Sync {
  fn integrate<'a>(
    &self,
    scene: &RayTraceScene<'a>,
    ray: Ray3,
  ) -> Color<f32, LinearRGBColorSpace<f32>>;

  fn default_sample_per_pixel(&self) -> usize {
    8
  }
}
