pub use rand as randx;
use rendiation_algebra::IntoNormalizedVector;
pub use rendiation_algebra::NormalizedVec3;
use rendiation_algebra::{InnerProductSpace, Vec2, Vec3};
pub use rendiation_geometry::*;

use crate::Sampler;

pub fn rand() -> f32 {
  randx::random()
}
