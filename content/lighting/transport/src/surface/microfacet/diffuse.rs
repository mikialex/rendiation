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
impl<C: IntersectionCtxBase> LightTransportSurface<C> for Diffuse<Lambertian> {
  fn bsdf(
    &self,
    _view_dir: NormalizedVec3<f32>,
    _light_dir: NormalizedVec3<f32>,
    _intersection: &C,
  ) -> Vec3<f32> {
    PhysicalDiffuse::albedo(self) / Vec3::splat(PI)
  }

  fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    _view_dir: NormalizedVec3<f32>,
    intersection: &C,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32> {
    // Simple cosine-sampling using Malley's method
    let sample = concentric_sample_disk(sampler.next_vec2());
    let x = sample.x;
    let y = sample.y;
    let z = (1.0 - x * x - y * y).sqrt();
    (intersection.shading_normal().local_to_world() * Vec3::new(x, y, z)).into_normalized()
  }

  fn pdf(
    &self,
    _view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    intersection: &C,
  ) -> f32 {
    light_dir.dot(intersection.shading_normal()).max(0.0) * INV_PI
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
