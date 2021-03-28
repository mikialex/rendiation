use crate::math::*;
use crate::Intersection;

pub mod physical;
pub use physical::*;

pub trait Material<G>: Send + Sync {
  /// sample the light input dir with brdf importance
  fn sample_light_dir(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
    geom: &G,
  ) -> NormalizedVec3;
  fn pdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
    geom: &G,
  ) -> f32;
  fn bsdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
    geom: &G,
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
