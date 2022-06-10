use crate::*;

pub mod specular_model;
pub use specular_model::*;
pub mod diffuse_model;
pub use diffuse_model::*;

use crate::Intersection;

#[derive(Clone)]
pub struct RtxPhysicalMaterial<D, S> {
  pub diffuse: D,
  pub specular: S,
}

#[derive(Clone)]
pub struct Specular<D, G, F> {
  pub roughness: f32,
  pub metallic: f32,
  pub ior: f32,
  pub normal_distribution_model: D,
  pub geometric_shadow_model: G,
  pub fresnel_model: F,
}

#[derive(Clone)]
pub struct Diffuse<T> {
  pub albedo: Vec3<f32>,
  pub diffuse_model: T,
}

pub trait MicroFacetNormalDistribution {
  /// Normal distribution term, which integral needs normalized to 1.
  fn d(&self, n: NormalizedVec3<f32>, h: NormalizedVec3<f32>) -> f32;

  /// sample a micro surface normal in normal's tangent space.
  fn sample_micro_surface_normal(
    &self,
    normal: NormalizedVec3<f32>,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32>;
  fn surface_normal_pdf(
    &self,
    normal: NormalizedVec3<f32>,
    sampled_normal: NormalizedVec3<f32>,
  ) -> f32;
}

pub trait MicroFacetGeometricShadow {
  fn g(&self, l: NormalizedVec3<f32>, v: NormalizedVec3<f32>, n: NormalizedVec3<f32>) -> f32;
}

pub trait MicroFacetFresnel {
  fn f(&self, v: NormalizedVec3<f32>, h: NormalizedVec3<f32>, f0: Vec3<f32>) -> Vec3<f32>;
}

// http://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html
// http://www.codinglabs.net/article_physically_based_rendering_cook_torrance.aspx
// https://blog.selfshadow.com/publications/s2012-shading-course/burley/s2012_pbs_disney_brdf_notes_v3.pdf
pub trait PhysicalSpecular:
  MicroFacetNormalDistribution + MicroFacetGeometricShadow + MicroFacetFresnel + Clone
{
  fn f0(&self, albedo: Vec3<f32>) -> Vec3<f32>;

  fn specular_estimate(&self, albedo: Vec3<f32>) -> f32;

  fn sample_light_dir_use_bsdf_importance(
    &self,
    view_dir: NormalizedVec3<f32>,
    intersection: &Intersection,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32> {
    let micro_surface_normal =
      self.sample_micro_surface_normal(intersection.shading_normal, sampler);
    view_dir.reverse().reflect(micro_surface_normal)
  }
  fn pdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    intersection: &Intersection,
  ) -> f32 {
    let micro_surface_normal = (view_dir + light_dir).into_normalized();
    let normal_pdf = self.surface_normal_pdf(intersection.shading_normal, micro_surface_normal);
    normal_pdf / (4.0 * micro_surface_normal.dot(view_dir).abs())
  }
}

impl<D, G, F> PhysicalSpecular for Specular<D, G, F>
where
  Self: MicroFacetNormalDistribution + MicroFacetGeometricShadow + MicroFacetFresnel + Clone,
{
  fn f0(&self, albedo: Vec3<f32>) -> Vec3<f32> {
    let f0 = ((self.ior - 1.0) / (self.ior + 1.0)).powi(2);
    Vec3::splat(f0).lerp(albedo, self.metallic)
  }

  fn specular_estimate(&self, albedo: Vec3<f32>) -> f32 {
    // Estimate specular contribution using Fresnel term
    fn mix_scalar<N: Scalar>(x: N, y: N, a: N) -> N {
      x * (N::one() - a) + y * a
    }
    let f0 = ((self.ior - 1.0) / (self.ior + 1.0)).powi(2);
    let f = (1.0 - self.metallic) * f0 + self.metallic * albedo.x; // albedo.mean()
    mix_scalar(f, 1.0, 0.2)
  }
}

pub trait PhysicalDiffuse: Material + Clone {
  fn albedo(&self) -> Vec3<f32>;
}

impl<D, S> Material for RtxPhysicalMaterial<D, S>
where
  D: PhysicalDiffuse + Send + Sync + 'static,
  S: PhysicalSpecular + Send + Sync + 'static,
{
  fn bsdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    intersection: &Intersection,
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
    specular + (Vec3::splat(1.0) - f) * diffuse
  }

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    view_dir: NormalizedVec3<f32>,
    intersection: &Intersection,
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
    intersection: &Intersection,
  ) -> f32 {
    let specular_estimate = self.specular.specular_estimate(self.diffuse.albedo());
    let spec = self.specular.pdf(view_dir, light_dir, intersection);
    let diff = self.diffuse.pdf(view_dir, light_dir, intersection);
    diff.lerp(spec, specular_estimate)
  }
}
