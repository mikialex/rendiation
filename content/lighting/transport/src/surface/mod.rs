use crate::*;

mod microfacet;
pub use microfacet::*;

pub struct ImportanceSampled<T, U> {
  pub sample: T,
  pub pdf: f32,
  pub importance: U,
}

pub type BRDFImportantSampled = ImportanceSampled<NormalizedVec3<f32>, Vec3<f32>>;

pub trait LightTransportSurface {
  fn bsdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    normal: NormalizedVec3<f32>,
  ) -> Vec3<f32>;

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    view_dir: NormalizedVec3<f32>,
    normal: NormalizedVec3<f32>,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32>;

  fn pdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    normal: NormalizedVec3<f32>,
  ) -> f32;

  fn sample_light_dir_use_bsdf_importance(
    &self,
    view_dir: NormalizedVec3<f32>,
    normal: NormalizedVec3<f32>,
    sampler: &mut dyn Sampler,
  ) -> BRDFImportantSampled {
    let light_dir = self.sample_light_dir_use_bsdf_importance_impl(view_dir, normal, sampler);
    ImportanceSampled {
      sample: light_dir,
      pdf: self.pdf(view_dir, light_dir, normal),
      importance: self.bsdf(view_dir, light_dir, normal),
    }
  }
}
