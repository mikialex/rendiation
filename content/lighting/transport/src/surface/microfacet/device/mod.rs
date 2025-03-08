mod direct_lighting;
pub use direct_lighting::*;

mod diffuse;
pub use diffuse::*;

mod specular;
pub use specular::*;

use crate::*;

pub struct ShaderRtxPhysicalMaterial<Diff, D, G, F> {
  pub diffuse: Diff,
  pub specular: ShaderSpecular<D, G, F>,
}

impl<Diff, D, G, F> ShaderLightTransportSurface for ShaderRtxPhysicalMaterial<Diff, D, G, F>
where
  Diff: ShaderPhysicalDiffuse + Send + Sync + 'static + ShaderLightTransportSurface,
  D: ShaderMicroFacetNormalDistribution,
  G: ShaderMicroFacetGeometricShadow,
  F: ShaderMicroFacetFresnel,
{
  fn bsdf(
    &self,
    view_dir: Node<Vec3<f32>>,
    light_dir: Node<Vec3<f32>>,
    normal: Node<Vec3<f32>>,
  ) -> Node<Vec3<f32>> {
    let specular = self.specular.bsdf(view_dir, light_dir, normal);
    let diffuse = self.diffuse.bsdf(view_dir, light_dir, normal);

    let estimate = self.specular.specular_estimate();
    estimate.mix(diffuse, specular)
  }

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    view_dir: Node<Vec3<f32>>,
    normal: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> Node<Vec3<f32>> {
    sampler
      .next()
      .less_than(self.specular.specular_estimate())
      .select_branched(
        || {
          self
            .specular
            .sample_light_dir_use_bsdf_importance_impl(view_dir, normal, sampler)
        },
        || {
          self
            .diffuse
            .sample_light_dir_use_bsdf_importance_impl(view_dir, normal, sampler)
        },
      )
  }

  fn pdf(
    &self,
    view_dir: Node<Vec3<f32>>,
    light_dir: Node<Vec3<f32>>,
    normal: Node<Vec3<f32>>,
  ) -> Node<f32> {
    let specular_estimate = self.specular.specular_estimate();
    let spec = self.specular.pdf(view_dir, light_dir, normal);
    let diff = self.diffuse.pdf(view_dir, light_dir, normal);
    specular_estimate.mix(diff, spec)
  }
}
