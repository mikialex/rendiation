use crate::*;

pub trait PhysicalDiffuse: Clone {
  fn albedo(&self) -> Vec3<f32>;
}

#[derive(Clone)]
pub struct Diffuse<T> {
  pub albedo: Vec3<f32>,
  pub diffuse_model: T,
}

#[derive(Clone)]
pub struct Lambertian;
impl LightTransportSurface for Diffuse<Lambertian> {
  type IntersectionCtx = ();

  fn bsdf(
    &self,
    _view_dir: NormalizedVec3<f32>,
    _light_dir: NormalizedVec3<f32>,
    _intersection: &Self::IntersectionCtx,
  ) -> Vec3<f32> {
    PhysicalDiffuse::albedo(self) / Vec3::splat(PI)
  }

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    _view_dir: NormalizedVec3<f32>,
    intersection: &Self::IntersectionCtx,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32> {
    // Simple cosine-sampling using Malley's method
    let sample = concentric_sample_disk(sampler);
    let x = sample.x;
    let y = sample.y;
    let z = (1.0 - x * x - y * y).sqrt();
    (intersection.shading_normal.local_to_world() * Vec3::new(x, y, z)).into_normalized()
  }

  fn pdf(
    &self,
    _view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    intersection: &Self::IntersectionCtx,
  ) -> f32 {
    light_dir.dot(intersection.shading_normal).max(0.0) * INV_PI
  }
}

impl PhysicalDiffuse for Diffuse<Lambertian> {
  fn albedo(&self) -> Vec3<f32> {
    self.albedo
  }
}

#[derive(Clone)]
pub struct OrenNayar {
  /// the standard deviation of the microfacet orientation angle
  /// in radians
  pub sigma: f32,
  pub albedo: Vec3<f32>,
  pub a: f32,
  pub b: f32,
}

impl OrenNayar {
  pub fn new(albedo: Vec3<f32>, sigma: f32) -> Self {
    let sigma2 = sigma * sigma;
    let a = 1. - (sigma2 / (2. * (sigma2 + 0.33)));
    let b = 0.45 * sigma2 / (sigma2 + 0.09);
    Self {
      sigma,
      albedo,
      a,
      b,
    }
  }
}

impl LightTransportSurface for OrenNayar {
  type IntersectionCtx = ();
  fn bsdf(
    &self,
    _view_dir: NormalizedVec3<f32>,
    _light_dir: NormalizedVec3<f32>,
    _intersection: &Self::IntersectionCtx,
  ) -> Vec3<f32> {
    todo!()
    // let sin_theta_i = sin_theta(wi);
    // let sin_theta_o = sin_theta(wo);
    // // compute cosine term of Oren-Nayar model
    // let max_cos = if sin_theta_i > 1.0e-4 && sin_theta_o > 1.0e-4 {
    //   let sin_phi_i = sin_phi(wi);
    //   let cos_phi_i = cos_phi(wi);
    //   let sin_phi_o = sin_phi(wo);
    //   let cos_phi_o = cos_phi(wo);
    //   let d_cos = cos_phi_i * cos_phi_o + sin_phi_i * sin_phi_o;
    //   d_cos.max(0.0 as Float)
    // } else {
    //   0.0 as Float
    // };
    // // compute sine and tangent terms of Oren-Nayar model
    // let sin_alpha;
    // let tan_beta = if abs_cos_theta(wi) > abs_cos_theta(wo) {
    //   sin_alpha = sin_theta_o;
    //   sin_theta_i / abs_cos_theta(wi)
    // } else {
    //   sin_alpha = sin_theta_i;
    //   sin_theta_o / abs_cos_theta(wo)
    // };
    // self.albedo * INV_PI * (self.a + self.b * max_cos * sin_alpha * tan_beta)
  }

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    _view_dir: NormalizedVec3<f32>,
    _intersection: &Self::IntersectionCtx,
    _sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32> {
    todo!()
  }

  fn pdf(
    &self,
    _view_dir: NormalizedVec3<f32>,
    _light_dir: NormalizedVec3<f32>,
    _intersection: &Self::IntersectionCtx,
  ) -> f32 {
    todo!()
  }
}
