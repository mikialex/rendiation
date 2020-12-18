use rendiation_math::{InnerProductSpace, Vector};

use crate::{
  math::{concentric_sample_disk, rand, rand2, Vec3, INV_PI, PI},
  MicroFacetFresnel, MicroFacetGeometricShadow, MicroFacetNormalDistribution,
  MicroSurfaceNormalSample, ScatteringEvent, Specular,
};

pub struct BlinnPhong;
impl<G, F> MicroFacetNormalDistribution for Specular<BlinnPhong, G, F> {
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
impl<G, F> MicroFacetNormalDistribution for Specular<Beckmann, G, F> {
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
impl<G, F> MicroFacetNormalDistribution for Specular<GGX, G, F> {
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
impl<D, F> MicroFacetGeometricShadow for Specular<D, CookTorrance, F> {
  fn g(&self, l: Vec3, v: Vec3, n: Vec3) -> f32 {
    let h = (l + v).normalize();
    let g = f32::min(n.dot(l) * n.dot(h), n.dot(v) * n.dot(h));
    let g = (2.0 * g) / v.dot(h);
    g.min(1.0)
  }
}

pub struct Schlick;
impl<D, G> MicroFacetFresnel for Specular<D, G, Schlick> {
  fn f(&self, v: Vec3, h: Vec3, f0: Vec3) -> Vec3 {
    f0 + (Vec3::splat(1.0) - f0) * (1.0 - v.dot(h)).powi(5)
  }
}
