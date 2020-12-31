use crate::{
  math::{concentric_sample_disk, rand, rand2},
  NormalizedVec3,
};
use rendiation_math::*;
use rendiation_render_entity::color::Color;

pub mod specular_model;
pub use specular_model::*;
pub mod diffuse_model;
pub use diffuse_model::*;

use crate::{
  math::{Vec3, PI},
  ray::Intersection,
  Material, INV_PI,
};

pub struct PhysicalMaterial<D, S> {
  pub diffuse: D,
  pub specular: S,
}

pub struct Specular<D, G, F> {
  pub roughness: f32,
  pub metallic: f32,
  pub ior: f32,
  pub normal_distribution_model: D,
  pub geometric_shadow_model: G,
  pub fresnel_model: F,
}

pub struct Diffuse<T> {
  pub albedo: Vec3,
  pub diffuse_model: T,
}

// this term need normalized to 1
pub trait MicroFacetNormalDistribution {
  fn d(&self, n: NormalizedVec3, h: NormalizedVec3) -> f32;

  /// sample a micro surface normal in normal's tangent space.
  fn sample_micro_surface_normal(&self, normal: NormalizedVec3) -> NormalizedVec3;
  fn surface_normal_pdf(&self, normal: NormalizedVec3, sampled_normal: NormalizedVec3) -> f32;
}

pub trait MicroFacetGeometricShadow {
  fn g(&self, l: NormalizedVec3, v: NormalizedVec3, n: NormalizedVec3) -> f32;
}

pub trait MicroFacetFresnel {
  fn f(&self, v: NormalizedVec3, h: NormalizedVec3, f0: Vec3) -> Vec3;
}

// http://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html
// http://www.codinglabs.net/article_physically_based_rendering_cook_torrance.aspx
// https://blog.selfshadow.com/publications/s2012-shading-course/burley/s2012_pbs_disney_brdf_notes_v3.pdf
pub trait PhysicalSpecular:
  MicroFacetNormalDistribution + MicroFacetGeometricShadow + MicroFacetFresnel
{
  fn bsdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
    albedo: Vec3,
  ) -> Vec3 {
    let l = light_dir;
    let v = view_dir;
    let n = intersection.hit_normal;
    let h = (l + v).into_normalized();

    (self.d(n, h) * self.g(l, v, n) * self.f(v, h, self.f0(albedo))) / (4.0 * n.dot(l) * n.dot(v))
  }

  fn f0(&self, albedo: Vec3) -> Vec3;

  fn sample_light_dir(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> NormalizedVec3 {
    let micro_surface_normal = self.sample_micro_surface_normal(intersection.hit_normal);
    view_dir.reverse().reflect(micro_surface_normal)
  }
  fn pdf(
    &self,
    view_dir: NormalizedVec3,
    micro_surface_normal: NormalizedVec3,
    intersection: &Intersection,
  ) -> f32 {
    let normal_pdf = self.surface_normal_pdf(view_dir, micro_surface_normal);
    normal_pdf / (4.0 * micro_surface_normal.dot(view_dir).abs())
  }
}

impl<D, G, F> PhysicalSpecular for Specular<D, G, F>
where
  Self: MicroFacetNormalDistribution + MicroFacetGeometricShadow + MicroFacetFresnel,
{
  fn f0(&self, albedo: Vec3) -> Vec3 {
    let f0 = ((self.ior - 1.0) / (self.ior + 1.0)).powi(2);
    Vec3::splat(f0).lerp(albedo, self.metallic)
  }
}

pub trait PhysicalDiffuse: Material {
  fn albedo(&self) -> Vec3;
}

impl<D, S> Material for PhysicalMaterial<D, S>
where
  D: PhysicalDiffuse + Send + Sync,
  S: PhysicalSpecular + Send + Sync,
{
  fn bsdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> Vec3 {
    let specular = self
      .specular
      .bsdf(view_dir, light_dir, intersection, self.diffuse.albedo());
    let diffuse = self.diffuse.bsdf(view_dir, light_dir, intersection);
    specular + diffuse
  }

  fn sample_light_dir(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> NormalizedVec3 {
    // if rand() > 0.5 {
    self.specular.sample_light_dir(view_dir, intersection)
    // } else {
    //   self.diffuse.sample_light_dir(view_dir, intersection)
    // }
  }

  fn pdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> f32 {
    self.specular.pdf(view_dir, light_dir, intersection) * 0.5
      + self.diffuse.pdf(view_dir, light_dir, intersection) * 0.5
  }
}
