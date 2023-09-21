use crate::*;

mod diffuse;
pub use diffuse::*;

mod specular;
pub use specular::*;

#[derive(Clone)]
pub struct RtxPhysicalMaterial<D, S> {
  pub diffuse: D,
  pub specular: S,
}

impl<D, S> LightTransportSurface for RtxPhysicalMaterial<D, S>
where
  D: PhysicalDiffuse + Send + Sync + 'static,
  S: PhysicalSpecular + Send + Sync + 'static,
{
  type IntersectionCtx = S::IntersectionCtx;
  fn bsdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    intersection: &Self::IntersectionCtx,
  ) -> Vec3<f32> {
    let l = light_dir;
    let v = view_dir;
    let n = intersection.shading_normal;
    let h = (l + v).into_normalized();

    let f = self
      .specular
      .f(v, h, self.specular.f0(self.diffuse.albedo()));

    let g = self.specular.g(l, v, n);

    let d = self.specular.d(n, h);

    let specular = (d * g * f) / (4.0 * n.dot(l) * n.dot(v));

    let diffuse = self.diffuse.bsdf(view_dir, light_dir, intersection);

    // i don't know why
    let estimate = self.specular.specular_estimate(self.diffuse.albedo());
    specular + (1.0 - estimate) * diffuse
  }

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    view_dir: NormalizedVec3<f32>,
    intersection: &Self::IntersectionCtx,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32> {
    if sampler.next() < self.specular.specular_estimate(self.diffuse.albedo()) {
      self
        .specular
        .sample_light_dir_use_bsdf_importance(view_dir, intersection, sampler)
    } else {
      self
        .diffuse
        .sample_light_dir_use_bsdf_importance_impl(view_dir, intersection, sampler)
    }
  }

  fn pdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    intersection: &Self::IntersectionCtx,
  ) -> f32 {
    let specular_estimate = self.specular.specular_estimate(self.diffuse.albedo());
    let spec = self.specular.pdf(view_dir, light_dir, intersection);
    let diff = self.diffuse.pdf(view_dir, light_dir, intersection);
    diff.lerp(spec, specular_estimate)
  }
}
