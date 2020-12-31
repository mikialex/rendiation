use crate::frame::*;
use crate::math::*;
use crate::ray::*;
use rendiation_render_entity::color::{Color, LinearRGBColorSpace, RGBColor};

pub mod physical;
pub use physical::*;

pub trait Material: Send + Sync {
  /// sample the light input dir with brdf importance
  fn sample_light_dir(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> NormalizedVec3;
  //  {
  //   let (out_dir, cos) = cosine_sample_hemisphere_in_dir(intersection.hit_normal);
  //   let pdf = cos / PI;
  //   ScatteringEvent { out_dir, pdf }.into()
  // }
  fn pdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> f32;
  fn bsdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> Vec3;
}
