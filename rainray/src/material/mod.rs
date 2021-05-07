use crate::Intersection;
use crate::{math::*, ImportanceSampled};

pub mod physical;
pub use physical::*;

pub trait RainrayMaterial: Send + Sync + 'static {
  /// sample the light input dir with brdf importance
  fn sample_light_dir_use_bsdf_importance(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> ImportanceSampled<NormalizedVec3> {
    let light_dir = self.sample_light_dir_use_bsdf_importance_impl(view_dir, intersection);
    ImportanceSampled {
      sample: light_dir,
      pdf: self.pdf(view_dir, light_dir, intersection),
    }
  }

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> NormalizedVec3;
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

pub trait Evaluation<C, I, O> {
  fn evaluate(&self, input: I, ctx: &C) -> O;
}

pub type Evaluator<C, I, O> = Box<dyn Evaluation<C, I, O>>;

pub struct ValueCell<O> {
  value: O,
}

impl<C, I, O: Copy> Evaluation<C, I, O> for ValueCell<O> {
  fn evaluate(&self, _: I, _: &C) -> O {
    self.value
  }
}

pub enum ValueOrTexture<O> {
  Value(O),
  // Texture()
}
