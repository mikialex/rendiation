use crate::{
  math::{concentric_sample_disk, rand, rand2},
  ScatteringEvent,
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

#[derive(Clone, Copy, Debug)]
pub struct MicroSurfaceNormalSample {
  pub micro_surface_normal: Vec3,
  pub pdf: f32,
}

// this term need normalized to 1
pub trait MicroFacetNormalDistribution {
  fn d(&self, n: Vec3, h: Vec3) -> f32;

  /// sample a micro surface normal in normal's tangent space.
  fn sample(&self, normal: Vec3) -> MicroSurfaceNormalSample;
}

pub trait MicroFacetGeometricShadow {
  fn g(&self, l: Vec3, v: Vec3, n: Vec3) -> f32;
}

pub trait MicroFacetFresnel {
  fn f(&self, v: Vec3, h: Vec3, f0: Vec3) -> Vec3;
}

// http://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html
// http://www.codinglabs.net/article_physically_based_rendering_cook_torrance.aspx
// https://blog.selfshadow.com/publications/s2012-shading-course/burley/s2012_pbs_disney_brdf_notes_v3.pdf
pub trait PhysicalSpecular:
  MicroFacetNormalDistribution + MicroFacetGeometricShadow + MicroFacetFresnel
{
  fn bsdf(
    &self,
    from_in_dir: Vec3,
    out_dir: Vec3,
    intersection: &Intersection,
    albedo: Vec3,
  ) -> Vec3 {
    let l = from_in_dir;
    let v = out_dir;
    let n = intersection.hit_normal;
    let h = (l + v).normalize();

    (self.d(n, h) * self.g(l, v, n) * self.f(v, h, self.f0(albedo))) / (4.0 * n.dot(l) * n.dot(v))
  }

  fn f0(&self, albedo: Vec3) -> Vec3;

  fn scatter(&self, in_dir: Vec3, intersection: &Intersection) -> Option<ScatteringEvent> {
    let MicroSurfaceNormalSample {
      micro_surface_normal,
      pdf,
    } = self.sample(intersection.hit_normal);

    let out_dir = in_dir.reflect(micro_surface_normal);
    let h = (in_dir + out_dir).normalize();
    ScatteringEvent {
      out_dir,
      pdf: pdf / (4.0 * h.dot(in_dir)),
    }
    .into()
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
  fn bsdf(&self, from_in_dir: Vec3, out_dir: Vec3, intersection: &Intersection) -> Vec3 {
    let specular = self
      .specular
      .bsdf(from_in_dir, out_dir, intersection, self.diffuse.albedo());
    let diffuse = self.diffuse.bsdf(from_in_dir, out_dir, intersection);
    specular + diffuse
  }

  // fn scatter(&self, in_dir: Vec3, intersection: &Intersection) -> Option<ScatteringEvent> {
  //   if rand() > 0.5 {
  //     self.specular.scatter(in_dir, intersection)
  //   } else {
  //     // sample diffuse
  //     todo!()
  //   }
  // }
}
