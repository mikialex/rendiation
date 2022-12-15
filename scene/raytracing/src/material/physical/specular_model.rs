use crate::*;

#[derive(Clone)]
pub struct BlinnPhong;
impl<G, F> MicroFacetNormalDistribution for Specular<BlinnPhong, G, F> {
  fn d(&self, n: NormalizedVec3<f32>, h: NormalizedVec3<f32>) -> f32 {
    let roughness_2 = self.roughness * self.roughness;
    let normalize_coefficient = 1. / (PI * roughness_2);
    let cos = n.dot(h).max(0.0);
    cos.powf(2.0 / roughness_2 - 2.0) * normalize_coefficient
  }
  // https://segmentfault.com/a/1190000000432254
  // https://www.cs.princeton.edu/courses/archive/fall16/cos526/papers/importance.pdf
  fn sample_micro_surface_normal(
    &self,
    normal: NormalizedVec3<f32>,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32> {
    let roughness_2 = self.roughness * self.roughness;
    let power = 2.0 / roughness_2 - 2.;
    let theta = sampler.next().powf(1.0 / (power + 1.0)).acos();

    let (sin_t, cos_t) = theta.sin_cos();
    // Generate halfway vector by sampling azimuth uniformly
    let sample = concentric_sample_disk(sampler);
    let x = sample.x;
    let y = sample.y;
    let h = Vec3::new(x * sin_t, y * sin_t, cos_t);
    (normal.local_to_world() * h).into_normalized()
  }

  fn surface_normal_pdf(
    &self,
    normal: NormalizedVec3<f32>,
    sampled_normal: NormalizedVec3<f32>,
  ) -> f32 {
    let roughness_2 = self.roughness * self.roughness;
    let power = 2.0 / roughness_2 - 2.;
    let cos_t = sampled_normal.dot(normal).abs();
    (power + 1.0) / (2.0 * PI) * cos_t.powf(power)
  }
}

#[derive(Clone)]
pub struct Beckmann;
impl<G, F> MicroFacetNormalDistribution for Specular<Beckmann, G, F> {
  fn d(&self, n: NormalizedVec3<f32>, h: NormalizedVec3<f32>) -> f32 {
    let cos_theta = n.dot(h);
    let nh2 = cos_theta * cos_theta;
    let m2 = self.roughness * self.roughness;
    ((nh2 - 1.0) / (m2 * nh2)).exp() / (m2 * PI * nh2 * nh2)
  }
  fn sample_micro_surface_normal(
    &self,
    normal: NormalizedVec3<f32>,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32> {
    // PIT for Beckmann distribution microfacet normal
    // θ = arctan √(-m^2 ln U)
    let m2 = self.roughness * self.roughness;
    let theta = (m2 * -sampler.next().ln()).sqrt().atan();

    let (sin_t, cos_t) = theta.sin_cos();
    // Generate halfway vector by sampling azimuth uniformly
    let sample = concentric_sample_disk(sampler);
    let x = sample.x;
    let y = sample.y;
    let h = Vec3::new(x * sin_t, y * sin_t, cos_t);
    (normal.local_to_world() * h).into_normalized()
  }

  fn surface_normal_pdf(
    &self,
    normal: NormalizedVec3<f32>,
    sampled_normal: NormalizedVec3<f32>,
  ) -> f32 {
    // p = 1 / (πm^2 cos^3 θ) * e^(-tan^2(θ) / m^2)
    let m2 = self.roughness * self.roughness;
    let cos_t = sampled_normal.dot(normal).abs();
    let sin_t = (1.0 - cos_t * cos_t).sqrt();
    (PI * m2 * cos_t.powi(3)).recip() * (-(sin_t / cos_t).powi(2) / m2).exp()
  }
}

#[derive(Clone)]
pub struct GGX;
impl<G, F> MicroFacetNormalDistribution for Specular<GGX, G, F> {
  fn d(&self, n: NormalizedVec3<f32>, h: NormalizedVec3<f32>) -> f32 {
    let cos_theta = n.dot(h);
    let cos_theta_2 = cos_theta * cos_theta;

    let root = self.roughness / (cos_theta_2 * (self.roughness * self.roughness - 1.) + 1.);
    INV_PI * (root * root)
  }

  // https://schuttejoe.github.io/post/ggximportancesamplingpart1/
  fn sample_micro_surface_normal(
    &self,
    _normal: NormalizedVec3<f32>,
    _sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32> {
    todo!()
  }

  fn surface_normal_pdf(
    &self,
    _normal: NormalizedVec3<f32>,
    _sampled_normal: NormalizedVec3<f32>,
  ) -> f32 {
    todo!()
  }
}

#[derive(Clone)]
pub struct CookTorrance;
impl<D, F> MicroFacetGeometricShadow for Specular<D, CookTorrance, F> {
  fn g(&self, l: NormalizedVec3<f32>, v: NormalizedVec3<f32>, n: NormalizedVec3<f32>) -> f32 {
    let h = (l + v).normalize();
    let g = f32::min(n.dot(l) * n.dot(h), n.dot(v) * n.dot(h));
    let g = (2.0 * g) / v.dot(h);
    g.min(1.0)
  }
}

#[derive(Clone)]
pub struct Schlick;
impl<D, G> MicroFacetFresnel for Specular<D, G, Schlick> {
  fn f(&self, v: NormalizedVec3<f32>, h: NormalizedVec3<f32>, f0: Vec3<f32>) -> Vec3<f32> {
    f0 + (Vec3::splat(1.0) - f0) * (1.0 - v.dot(h)).powi(5)
  }
}
