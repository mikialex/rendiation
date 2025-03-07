use crate::*;

pub trait PhysicalDiffuse: Clone {
  fn albedo(&self) -> Vec3<f32>;
}

#[derive(Clone)]
pub struct Lambertian {
  pub albedo: Vec3<f32>,
}
impl PhysicalDiffuse for Lambertian {
  fn albedo(&self) -> Vec3<f32> {
    self.albedo
  }
}

impl LightTransportSurface for Lambertian {
  fn bsdf(
    &self,
    _view_dir: NormalizedVec3<f32>,
    _light_dir: NormalizedVec3<f32>,
    _normal: NormalizedVec3<f32>,
  ) -> Vec3<f32> {
    PhysicalDiffuse::albedo(self) / Vec3::splat(PI)
  }

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    _view_dir: NormalizedVec3<f32>,
    normal: NormalizedVec3<f32>,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32> {
    // Simple cosine-sampling using Malley's method
    let sample = concentric_sample_disk(sampler.next_vec2());
    let x = sample.x;
    let y = sample.y;
    let z = (1.0 - x * x - y * y).sqrt();
    (normal.local_to_world() * Vec3::new(x, y, z)).into_normalized()
  }

  fn pdf(
    &self,
    _view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    normal: NormalizedVec3<f32>,
  ) -> f32 {
    light_dir.dot(normal).max(0.0) * INV_PI
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
impl PhysicalDiffuse for OrenNayar {
  fn albedo(&self) -> Vec3<f32> {
    self.albedo
  }
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
