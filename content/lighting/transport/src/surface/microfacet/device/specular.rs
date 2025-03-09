use rendiation_shader_library::sampling::{concentric_sample_disk_device_fn, tbn_fn};

use crate::*;

pub struct ShaderSpecular<D, G, F> {
  pub f0: Node<Vec3<f32>>,
  pub normal_distribution_model: D,
  pub geometric_shadow_model: G,
  pub fresnel_model: F,
}

impl<D, G, F> ShaderSpecular<D, G, F>
where
  D: ShaderMicroFacetNormalDistribution,
  G: ShaderMicroFacetGeometricShadow,
  F: ShaderMicroFacetFresnel,
{
  pub fn specular_estimate(&self) -> Node<f32> {
    let f0 = self.f0.x().max(self.f0.y()).max(self.f0.z());
    f0.mix(0.2, 1.0)
  }

  pub fn bsdf(
    &self,
    view_dir: Node<Vec3<f32>>,
    light_dir: Node<Vec3<f32>>,
    normal: Node<Vec3<f32>>,
  ) -> Node<Vec3<f32>> {
    let l = light_dir;
    let v = view_dir;
    let n = normal;
    let h = (l + v).normalize();

    let f = self.fresnel_model.f(v, h, self.f0);

    let g = self.geometric_shadow_model.g(l, v, n);

    let d = self.normal_distribution_model.d(n, h);

    let n_dot_l = l.dot(n).max(EPSILON_SHADING);
    let n_dot_v = v.dot(n).max(EPSILON_SHADING);

    (d * g * f) / (val(4.0) * n_dot_l * n_dot_v).splat()
  }

  pub fn sample_light_dir_use_bsdf_importance_impl(
    &self,
    view_dir: Node<Vec3<f32>>,
    normal: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> Node<Vec3<f32>> {
    let micro_surface_normal = self
      .normal_distribution_model
      .sample_micro_surface_normal(normal, sampler);
    micro_surface_normal.reflect(-view_dir)
  }

  pub fn pdf(
    &self,
    view_dir: Node<Vec3<f32>>,
    light_dir: Node<Vec3<f32>>,
    normal: Node<Vec3<f32>>,
  ) -> Node<f32> {
    let micro_surface_normal = (view_dir + light_dir).normalize();
    let normal_pdf = self
      .normal_distribution_model
      .surface_normal_pdf(normal, micro_surface_normal);
    normal_pdf / (val(4.0) * micro_surface_normal.dot(view_dir).abs())
  }
}

pub trait ShaderMicroFacetNormalDistribution {
  /// Normal distribution term, which integral needs normalized to 1.
  fn d(&self, n: Node<Vec3<f32>>, h: Node<Vec3<f32>>) -> Node<f32>;

  /// sample a micro surface normal in normal's tangent space.
  fn sample_micro_surface_normal(
    &self,
    normal: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> Node<Vec3<f32>>;
  fn surface_normal_pdf(
    &self,
    normal: Node<Vec3<f32>>,
    sampled_normal: Node<Vec3<f32>>,
  ) -> Node<f32>;
}

pub trait ShaderMicroFacetGeometricShadow {
  fn g(&self, l: Node<Vec3<f32>>, v: Node<Vec3<f32>>, n: Node<Vec3<f32>>) -> Node<f32>;
}

pub trait ShaderMicroFacetFresnel {
  fn f(&self, v: Node<Vec3<f32>>, h: Node<Vec3<f32>>, f0: Node<Vec3<f32>>) -> Node<Vec3<f32>>;
}

/// Microfacet Models for Refraction through Rough Surfaces - equation (33)
/// http://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html
pub struct ShaderGGX {
  pub roughness: Node<f32>,
}

const EPSILON_SHADING: f32 = 0.0001;
impl ShaderMicroFacetNormalDistribution for ShaderGGX {
  fn d(&self, n: Node<Vec3<f32>>, h: Node<Vec3<f32>>) -> Node<f32> {
    let n_o_h = n.dot(h).max(EPSILON_SHADING);
    let roughness2 = self.roughness * self.roughness;
    let d = (n_o_h * roughness2 - n_o_h) * n_o_h + val(1.0);
    roughness2 / (val(f32::PI()) * d * d)
  }

  fn sample_micro_surface_normal(
    &self,
    normal: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> Node<Vec3<f32>> {
    let sample = sampler.next();
    let theta = (self.roughness * (sample / (val(1.0) - sample)).sqrt()).atan();

    let sin_t = theta.sin();
    let cos_t = theta.cos();
    // Generate halfway vector by sampling azimuth uniformly
    let sample = concentric_sample_disk_device_fn(sampler.next_2d());
    let x = sample.x();
    let y = sample.y();
    let h = (x * sin_t, y * sin_t, cos_t).into();
    tbn_fn(normal) * h
  }

  fn surface_normal_pdf(
    &self,
    normal: Node<Vec3<f32>>,
    sampled_normal: Node<Vec3<f32>>,
  ) -> Node<f32> {
    let cos = normal.dot(sampled_normal);
    let sin = (val(1.0) - cos * cos).sqrt();
    sin * cos * self.d(normal, sampled_normal)
  }
}

pub struct ShaderBeckmann {
  pub roughness: Node<f32>,
}

impl ShaderMicroFacetNormalDistribution for ShaderBeckmann {
  fn d(&self, n: Node<Vec3<f32>>, h: Node<Vec3<f32>>) -> Node<f32> {
    let cos_theta = n.dot(h).max(EPSILON_SHADING);
    let nh2 = cos_theta * cos_theta;
    let m2 = self.roughness * self.roughness;
    ((nh2 - val(1.0)) / (m2 * nh2)).exp() / (m2 * val(f32::PI()) * nh2 * nh2)
  }

  fn sample_micro_surface_normal(
    &self,
    normal: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> Node<Vec3<f32>> {
    // PIT for Beckmann distribution microfacet normal
    // θ = arctan √(-m^2 ln U)
    let m2 = self.roughness * self.roughness;
    let theta = (m2 * -sampler.next().ln()).sqrt().atan();

    let sin_t = theta.sin();
    let cos_t = theta.cos();
    // Generate halfway vector by sampling azimuth uniformly
    let sample = concentric_sample_disk_device_fn(sampler.next_2d());
    let x = sample.x();
    let y = sample.y();
    let h = (x * sin_t, y * sin_t, cos_t).into();
    tbn_fn(normal) * h
  }

  fn surface_normal_pdf(
    &self,
    normal: Node<Vec3<f32>>,
    sampled_normal: Node<Vec3<f32>>,
  ) -> Node<f32> {
    // p = 1 / (πm^2 cos^3 θ) * e^(-tan^2(θ) / m^2)
    let m2 = self.roughness * self.roughness;
    let cos_t = sampled_normal.dot(normal).abs();
    let sin_t = (val(1.0) - cos_t * cos_t).sqrt();
    (-(sin_t / cos_t).pow(2.) / m2).exp() / (val(f32::PI()) * m2 * cos_t.pow(3.))
  }
}

/// https://de45xmedrsdbp.cloudfront.net/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
#[derive(Clone)]
pub struct ShaderSchlick;
impl ShaderMicroFacetFresnel for ShaderSchlick {
  fn f(&self, v: Node<Vec3<f32>>, h: Node<Vec3<f32>>, f0: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
    f0 + (val(Vec3::splat(1.0)) - f0) * (val(1.0) - v.dot(h)).pow(5.)
  }
}

/// Moving Frostbite to Physically Based Rendering 3.0 - page 12, listing 2
/// https://seblagarde.files.wordpress.com/2015/07/course_notes_moving_frostbite_to_pbr_v32.pdf
pub struct ShaderSmithGGXCorrelatedGeometryShadow {
  pub roughness: Node<f32>,
}
impl ShaderMicroFacetGeometricShadow for ShaderSmithGGXCorrelatedGeometryShadow {
  fn g(&self, l: Node<Vec3<f32>>, v: Node<Vec3<f32>>, n: Node<Vec3<f32>>) -> Node<f32> {
    let n_dot_l = l.dot(n).max(EPSILON_SHADING);
    let n_dot_v = v.dot(n).max(EPSILON_SHADING);
    let roughness2 = self.roughness * self.roughness;

    let vis_smith_v = n_dot_v * (n_dot_v * (n_dot_v - n_dot_v * roughness2) + roughness2).sqrt();
    let vis_smith_l = n_dot_l * (n_dot_l * (n_dot_l - n_dot_l * roughness2) + roughness2).sqrt();
    let g = val(0.5) / (vis_smith_v + vis_smith_l);

    // the above code has already divided by 4 * n_dot_l * n_dot_v for optimization(see pdf)
    // here we simply multiply back, because we do division outside...
    // todo, improve this
    g * val(4.0) * n_dot_l * n_dot_v
  }
}
