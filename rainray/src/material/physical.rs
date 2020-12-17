use crate::math::{concentric_sample_disk, rand, rand2, PI};
use rendiation_math::*;
use rendiation_render_entity::color::Color;

use crate::{
  math::{Vec3, PI},
  ray::Intersection,
  Material, INV_PI,
};

// http://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html
// http://www.codinglabs.net/article_physically_based_rendering_cook_torrance.aspx
// https://blog.selfshadow.com/publications/s2012-shading-course/burley/s2012_pbs_disney_brdf_notes_v3.pdf
impl<T> Material for T
where
  T: MicroFacetNormalDistribution
    + MicroFacetGeometricShadow
    + MicroFacetFresnel
    + LambertianBase
    + Send
    + Sync,
{
  fn bsdf(&self, from_in_dir: Vec3, out_dir: Vec3, intersection: &Intersection) -> Vec3 {
    let l = from_in_dir;
    let v = out_dir;
    let n = intersection.hit_normal;
    let h = (l + v).normalize();
    let specular = (self.d(n, h) * self.g(l, v, n) * self.f(v, h)) / (4.0 * n.dot(l) * n.dot(v));
    let diffuse = self.albedo() / Vec3::splat(PI);
    specular + diffuse
  }
}

pub trait LambertianBase {
  fn albedo(&self) -> Vec3;
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
  fn f(&self, v: Vec3, h: Vec3) -> Vec3;
}

pub fn saturate(v: f32) -> f32 {
  v.min(1.0).max(0.0)
}

pub struct PhysicalMaterial<D, G, F> {
  pub albedo: Vec3,
  pub roughness: f32,
  pub metallic: f32,
  pub ior: f32,
  pub normal_distribution_model: D,
  pub geometric_shadow_model: G,
  pub fresnel_model: F,
}

impl<D, G, F> LambertianBase for PhysicalMaterial<D, G, F> {
  fn albedo(&self) -> Vec3 {
    self.albedo
  }
}

impl<D, F, G> PhysicalMaterial<D, G, F> {
  pub fn f0(&self) -> Vec3 {
    let f0 = ((self.ior - 1.0) / (self.ior + 1.0)).powi(2);
    Vec3::splat(f0).lerp(self.albedo, self.metallic)
  }
}

pub struct BlinnPhong;
impl<G, F> MicroFacetNormalDistribution for PhysicalMaterial<BlinnPhong, G, F> {
  fn d(&self, n: Vec3, h: Vec3) -> f32 {
    let roughness_2 = self.roughness * self.roughness;
    let normalize_coefficient = 1. / (PI * roughness_2);
    let cos = n.dot(h).max(0.0);
    cos.powf(2.0 / roughness_2 - 2.0) * normalize_coefficient
  }
  fn sample(&self, normal: Vec3) -> MicroSurfaceNormalSample {
    todo!()
  }
}

pub struct Beckmann;
impl<G, F> MicroFacetNormalDistribution for PhysicalMaterial<Beckmann, G, F> {
  fn d(&self, n: Vec3, h: Vec3) -> f32 {
    let cos_theta = n.dot(h);
    let nh2 = cos_theta * cos_theta;
    let m2 = self.roughness * self.roughness;
    ((nh2 - 1.0) / (m2 * nh2)).exp() / (m2 * PI * nh2 * nh2)
  }
  fn sample(&self, normal: Vec3) -> MicroSurfaceNormalSample {
    // PIT for Beckmann distribution microfacet normal
    // θ = arctan √(-m^2 ln U)
    let m2 = self.roughness * self.roughness;
    let theta = (m2 * -rand().ln()).sqrt().atan();
    let (sin_t, cos_t) = theta.sin_cos();

    // Generate halfway vector by sampling azimuth uniformly
    let disk_sample = concentric_sample_disk(rand2());
    let micro_surface_normal = Vec3::new(disk_sample.x * sin_t, disk_sample.y * sin_t, cos_t);

    // p = 1 / (πm^2 cos^3 θ) * e^(-tan^2(θ) / m^2)
    let pdf = (PI * m2 * cos_t.powi(3)).recip() * (-(sin_t / cos_t).powi(2) / m2).exp();

    MicroSurfaceNormalSample {
      micro_surface_normal,
      pdf,
    }
  }
}

pub struct GGX;
impl<G, F> MicroFacetNormalDistribution for PhysicalMaterial<GGX, G, F> {
  fn d(&self, n: Vec3, h: Vec3) -> f32 {
    let cos_theta = n.dot(h);
    let cos_theta_2 = cos_theta * cos_theta;

    let root = self.roughness / (cos_theta_2 * (self.roughness * self.roughness - 1.) + 1.);
    INV_PI * (root * root)
  }
  fn sample(&self, normal: Vec3) -> MicroSurfaceNormalSample {
    todo!()
  }
}

pub struct CookTorrance;
impl<D, F> MicroFacetGeometricShadow for PhysicalMaterial<D, CookTorrance, F> {
  fn g(&self, l: Vec3, v: Vec3, n: Vec3) -> f32 {
    let h = (l + v).normalize();
    let g = f32::min(n.dot(l) * n.dot(h), n.dot(v) * n.dot(h));
    let g = (2.0 * g) / v.dot(h);
    g.min(1.0)
  }
}

pub struct Schlick;
impl<D, G> MicroFacetFresnel for PhysicalMaterial<D, G, Schlick> {
  fn f(&self, v: Vec3, h: Vec3) -> Vec3 {
    let f0 = self.f0();
    f0 + (Vec3::splat(1.0) - f0) * (1.0 - v.dot(h)).powi(5)
  }
}
