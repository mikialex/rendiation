use crate::*;

#[derive(Clone)]
pub struct Specular<D, G, F> {
  pub roughness: f32,
  pub metallic: f32,
  pub ior: f32,
  pub normal_distribution_model: D,
  pub geometric_shadow_model: G,
  pub fresnel_model: F,
}

// http://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html
// http://www.codinglabs.net/article_physically_based_rendering_cook_torrance.aspx
// https://blog.selfshadow.com/publications/s2012-shading-course/burley/s2012_pbs_disney_brdf_notes_v3.pdf
pub trait PhysicalSpecular<C: IntersectionCtxBase>:
  MicroFacetNormalDistribution + MicroFacetGeometricShadow + MicroFacetFresnel + Clone
{
  fn f0(&self, albedo: Vec3<f32>) -> Vec3<f32>;

  fn specular_estimate(&self, albedo: Vec3<f32>) -> f32 {
    // Estimate specular contribution using Fresnel term
    fn mix_scalar<N: Scalar>(x: N, y: N, a: N) -> N {
      x * (N::one() - a) + y * a
    }
    let f0 = self.f0(albedo).max_channel();
    mix_scalar(f0, 1.0, 0.2)
  }

  fn sample_light_dir_use_bsdf_importance(
    &self,
    view_dir: NormalizedVec3<f32>,
    intersection: &C,
    sampler: &mut dyn Sampler,
  ) -> NormalizedVec3<f32> {
    let micro_surface_normal =
      self.sample_micro_surface_normal(intersection.shading_normal(), sampler);
    view_dir.reverse().reflect(micro_surface_normal)
  }
  fn pdf(
    &self,
    view_dir: NormalizedVec3<f32>,
    light_dir: NormalizedVec3<f32>,
    intersection: &C,
  ) -> f32 {
    let micro_surface_normal = (view_dir + light_dir).into_normalized();
    let normal_pdf = self.surface_normal_pdf(intersection.shading_normal(), micro_surface_normal);
    normal_pdf / (4.0 * micro_surface_normal.dot(view_dir).abs())
  }
}

impl<D, G, F, C> PhysicalSpecular<C> for Specular<D, G, F>
where
  Self: MicroFacetNormalDistribution + MicroFacetGeometricShadow + MicroFacetFresnel + Clone,
  C: IntersectionCtxBase,
{
  fn f0(&self, albedo: Vec3<f32>) -> Vec3<f32> {
    let f0 = ((self.ior - 1.0) / (self.ior + 1.0)).powi(2);
    Vec3::splat(f0).lerp(albedo, self.metallic)
  }
}

pub trait MicroFacetNormalDistribution {
  /// Normal distribution term, which integral needs normalized to 1.
  fn d(&self, n: NormalizedVec3<f32>, h: NormalizedVec3<f32>) -> f32;

  /// sample a micro surface normal in normal's tangent space.
  fn sample_micro_surface_normal(
    &self,
    normal: NormalizedVec3<f32>,
    sample: &mut dyn Sampler,
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
