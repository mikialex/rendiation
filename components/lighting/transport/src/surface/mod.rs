use crate::*;

mod microfacet;
pub use microfacet::*;

pub struct ImportanceSampled<T, U> {
  pub sample: T,
  pub pdf: f32,
  pub importance: U,
}

pub type BRDFImportantSampled = ImportanceSampled<NormalizedVec3<f32>, Vec3<f32>>;

pub trait LightTransportSurface<C> {
  fn bsdf(
    &self,
    _view_dir: NormalizedVec3<f32>,
    _light_dir: NormalizedVec3<f32>,
    _intersection: &C,
  ) -> Vec3<f32>;

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    _view_dir: NormalizedVec3<f32>,
    intersection: &C,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32>;

  fn pdf(
    &self,
    _view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    intersection: &C,
  ) -> f32;

  fn sample_light_dir_use_bsdf_importance(
    &self,
    view_dir: NormalizedVec3<f32>,
    intersection: &C,
    sampler: &mut dyn Sampler,
  ) -> BRDFImportantSampled {
    let light_dir = self.sample_light_dir_use_bsdf_importance_impl(view_dir, intersection, sampler);
    ImportanceSampled {
      sample: light_dir,
      pdf: self.pdf(view_dir, light_dir, intersection),
      importance: self.bsdf(view_dir, light_dir, intersection),
    }
  }
}

pub trait IntersectionCtxBase {
  fn shading_normal(&self) -> NormalizedVec3<f32>;
}
