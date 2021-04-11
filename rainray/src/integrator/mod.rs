use rendiation_color::{Color, LinearRGBColorSpace};
use rendiation_geometry::Ray3;

use crate::{scene::Scene, SceneCache};

pub mod ao;
pub mod path_trace;
pub use ao::*;
pub use path_trace::*;

pub trait Integrator: Sync {
  fn integrate<'a>(
    &self,
    scene: &'a Scene,
    scene_cache: &SceneCache<'a>,
    ray: Ray3,
  ) -> Color<f32, LinearRGBColorSpace<f32>>;
}
