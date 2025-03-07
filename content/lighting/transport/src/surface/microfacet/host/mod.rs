use crate::*;

mod diffuse;
pub use diffuse::*;

mod specular;
pub use specular::*;

#[derive(Clone)]
pub struct RtxPhysicalMaterial<Diff, D, G, F> {
  pub diffuse: Diff,
  pub specular: Specular<D, G, F>,
}

impl<Diff, D, G, F> LightTransportSurface for RtxPhysicalMaterial<Diff, D, G, F>
where
  Diff: PhysicalDiffuse + Send + Sync + 'static + LightTransportSurface,
  D: MicroFacetNormalDistribution,
  G: MicroFacetGeometricShadow,
  F: MicroFacetFresnel,
{
  fn bsdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    normal: NormalizedVec3<f32>,
  ) -> Vec3<f32> {
    let specular = self
      .specular
      .bsdf(view_dir, light_dir, normal, self.diffuse.albedo());
    let diffuse = self.diffuse.bsdf(view_dir, light_dir, normal);

    let estimate = self.specular.specular_estimate(self.diffuse.albedo());
    specular + (1.0 - estimate) * diffuse
  }

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    view_dir: NormalizedVec3<f32>,
    normal: NormalizedVec3<f32>,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32> {
    if sampler.next() < self.specular.specular_estimate(self.diffuse.albedo()) {
      self
        .specular
        .sample_light_dir_use_bsdf_importance_impl(view_dir, normal, sampler)
    } else {
      self
        .diffuse
        .sample_light_dir_use_bsdf_importance_impl(view_dir, normal, sampler)
    }
  }

  fn pdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    normal: NormalizedVec3<f32>,
  ) -> f32 {
    let specular_estimate = self.specular.specular_estimate(self.diffuse.albedo());
    let spec = self.specular.pdf(view_dir, light_dir, normal);
    let diff = self.diffuse.pdf(view_dir, light_dir, normal);
    diff.lerp(spec, specular_estimate)
  }
}
