use crate::{math::*, ImportanceSampled};
use crate::{Intersection, Sampler};

pub mod physical;
pub use physical::*;
use rendiation_algebra::Vec3;

pub trait Material: Send + Sync + 'static + dyn_clone::DynClone {
  /// sample the light input dir with brdf importance
  fn sample_light_dir_use_bsdf_importance(
    &self,
    view_dir: NormalizedVec3<f32>,
    intersection: &Intersection,
    sampler: &mut dyn Sampler,
  ) -> ImportanceSampled<NormalizedVec3<f32>> {
    let light_dir = self.sample_light_dir_use_bsdf_importance_impl(view_dir, intersection, sampler);
    ImportanceSampled {
      sample: light_dir,
      pdf: self.pdf(view_dir, light_dir, intersection),
    }
  }

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    view_dir: NormalizedVec3<f32>,
    intersection: &Intersection,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32>;
  fn pdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    intersection: &Intersection,
  ) -> f32;
  fn bsdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    intersection: &Intersection,
  ) -> Vec3<f32>;
}

dyn_clone::clone_trait_object!(Material);

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
