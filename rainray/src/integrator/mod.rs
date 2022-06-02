use rendiation_color::LinearRGBColor;
use rendiation_geometry::Ray3;
use rendiation_scene_raytracing::RayTracingScene;

use crate::Scene;

pub mod ao;
pub mod intersection_stat;
pub mod path_trace;
pub use ao::*;
pub use intersection_stat::*;
pub use path_trace::*;

pub trait Integrator: Sync {
  fn integrate(&self, scene: &Scene<RayTracingScene>, ray: Ray3) -> LinearRGBColor<f32>;

  fn default_sample_per_pixel(&self) -> usize {
    8
  }
}
