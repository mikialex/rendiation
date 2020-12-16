use rendiation_math::*;

use crate::{
  math::{Vec3, PI},
  INV_PI,
};

// http://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html
// http://www.codinglabs.net/article_physically_based_rendering_cook_torrance.aspx
// https://blog.selfshadow.com/publications/s2012-shading-course/burley/s2012_pbs_disney_brdf_notes_v3.pdf
// https://blog.uwa4d.com/archives/1582.html
pub trait MicroFacetBRDF:
  MicroFacetNormalDistribution + MicroFacetGeometricShadow + MicroFacetFresnel
{
  fn evaluate(&self, l: Vec3, v: Vec3, n: Vec3) -> Vec3 {
    let h = (l + v).normalize();
    (self.d(n, h) * self.g(l, v, n) * self.f(v, h)) / (4.0 * n.dot(l) * n.dot(v))
  }
}

impl<T> MicroFacetBRDF for T where
  T: MicroFacetNormalDistribution + MicroFacetGeometricShadow + MicroFacetFresnel
{
}

// this term need normalized to 1
pub trait MicroFacetNormalDistribution {
  fn d(&self, n: Vec3, h: Vec3) -> f32;
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

impl<D, F, G> PhysicalMaterial<D, G, F> {
  pub fn f0(&self) -> Vec3 {
    let f0 = ((self.ior - 1.0) / (self.ior + 1.0)).powi(2);
    Vec3::splat(f0).lerp(self.albedo, self.metallic)
  }
}

pub struct BlinnPhong;
impl<G, F> MicroFacetNormalDistribution for PhysicalMaterial<BlinnPhong, G, F> {
  fn d(&self, n: Vec3, h: Vec3) -> f32 {
    let normalize_coefficient = (self.roughness + 2.0) / 2.0 * PI;
    let cos = n.dot(h);
    saturate(cos).powf(self.roughness) * normalize_coefficient
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
}

pub struct GGX;
impl<G, F> MicroFacetNormalDistribution for PhysicalMaterial<GGX, G, F> {
  fn d(&self, n: Vec3, h: Vec3) -> f32 {
    let cos_theta = n.dot(h);
    let cos_theta_2 = cos_theta * cos_theta;

    let root = self.roughness / (cos_theta_2 * (self.roughness * self.roughness - 1.) + 1.);
    INV_PI * (root * root)
  }
}

pub struct Schlick;
impl<D, G> MicroFacetFresnel for PhysicalMaterial<D, G, Schlick> {
  fn f(&self, v: Vec3, h: Vec3) -> Vec3 {
    let f0 = self.f0();
    f0 + (Vec3::splat(1.0) - f0) * (1.0 - v.dot(h)).powi(5)
  }
}
