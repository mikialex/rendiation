use rendiation_math::Vec2;
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
  fn integrate(
    &self,
    camera: &Camera,
    scene: &Scene,
    frame_size: Vec2<usize>,
    current: Vec2<usize>,
  ) -> Color<LinearRGBColorSpace<f32>>;
}
