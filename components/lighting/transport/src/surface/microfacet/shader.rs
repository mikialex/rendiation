use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderPhysicalShading {
  pub diffuse: Vec3<f32>,
  pub perceptual_roughness: f32,
  pub f0: Vec3<f32>,
}

pub fn bias_n_dot_l(n_dot_l: Node<f32>) -> Node<f32> {
  (n_dot_l * val(1.08) - val(0.08)).saturate()
}

/// Microfacet Models for Refraction through Rough Surfaces - equation (33)
/// http://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html
fn d_ggx(n_o_h: Node<f32>, roughness4: Node<f32>) -> Node<f32> {
  let d = (n_o_h * roughness4 - n_o_h) * n_o_h + val(1.0);
  roughness4 / (val(f32::PI()) * d * d)
}

// NOTE: Basically same as
// https://de45xmedrsdbp.cloudfront.net/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
// However, calculate a F90 instead of using 1.0 directly
#[shader_fn]
fn fresnel(v_dot_h: Node<f32>, f0: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  let fc = (val(1.0) - v_dot_h).pow(5.0);
  let f90 = (f0 * val(50.0)).clamp(Vec3::zero(), Vec3::one());
  f90 * fc + f0 * (val(1.0) - fc)
}

/// Moving Frostbite to Physically Based Rendering 3.0 - page 12, listing 2
/// https://seblagarde.files.wordpress.com/2015/07/course_notes_moving_frostbite_to_pbr_v32.pdf
fn v_smith_correlated(n_dot_l: Node<f32>, n_dot_v: Node<f32>, roughness4: Node<f32>) -> Node<f32> {
  let vis_smith_v = n_dot_v * (n_dot_v * (n_dot_v - n_dot_v * roughness4) + roughness4).sqrt();
  let vis_smith_l = n_dot_l * (n_dot_l * (n_dot_l - n_dot_l * roughness4) + roughness4).sqrt();
  val(0.5) / (vis_smith_v + vis_smith_l)
}

pub fn evaluate_brdf_diffuse(diffuse_color: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  val(1. / f32::PI()) * diffuse_color
}

pub fn evaluate_brdf_specular(
  shading: ENode<ShaderPhysicalShading>,
  v: Node<Vec3<f32>>,
  l: Node<Vec3<f32>>,
  n: Node<Vec3<f32>>,
) -> Node<Vec3<f32>> {
  const EPSILON_SHADING: f32 = 0.0001;

  let h = (l + v).normalize();
  let n_dot_l = l.dot(n).max(0.0);
  let n_dot_v = n.dot(v).max(EPSILON_SHADING);
  let n_dot_h = n.dot(h).max(EPSILON_SHADING);
  let v_hot_h = v.dot(h).max(EPSILON_SHADING);

  let roughness2 = shading.perceptual_roughness;
  let roughness4 = roughness2 * roughness2;

  let f = fresnel(v_hot_h, shading.f0);
  let d = d_ggx(n_dot_h, roughness4).max(0.0);
  let g = v_smith_correlated(n_dot_l, n_dot_v, roughness4).max(0.0);

  d * g * f
}

use rendiation_shader_library::sampling::hammersley_2d_fn;

/// for ibl
pub fn integrate_brdf(
  roughness: Node<f32>, // perceptual roughness
  n_dot_v: Node<f32>,
  sample_count: Node<u32>,
) -> Node<Vec2<f32>> {
  let roughness2 = roughness * roughness;
  let view = vec3_node(((val(1.) - n_dot_v * n_dot_v).sqrt(), val(0.), n_dot_v));

  let sum = sample_count
    .into_shader_iter()
    .map(|index| {
      let random = hammersley_2d_fn(index, sample_count);
      let half = hemisphere_importance_sample_dggx(random, roughness2);

      let light = val(2.0) * view.dot(half) * half - view;
      let n_dot_l = light.z().saturate();
      let n_dot_h = half.z().saturate();
      let v_dot_h = view.dot(half).saturate();

      let g = g_smith(n_dot_l, n_dot_v, roughness2);
      let g_vis = (g * v_dot_h / (n_dot_h * n_dot_v)).max(0.);
      let fc = (val(1.) - v_dot_h).pow(5.0);

      vec2_node(((val(1.) - fc) * g_vis, fc * g_vis))
    })
    .sum();

  sum / sample_count.into_f32().splat()
}

pub fn g1_ggx_schlick(n_dot_v: Node<f32>, a: Node<f32>) -> Node<f32> {
  let k = a / val(2.);
  n_dot_v.max(val(0.001)) / (n_dot_v * (val(1.0) - k) + k)
}

pub fn g_smith(n_dot_v: Node<f32>, n_dot_l: Node<f32>, a: Node<f32>) -> Node<f32> {
  g1_ggx_schlick(n_dot_l, a) * g1_ggx_schlick(n_dot_v, a)
}

pub fn hemisphere_importance_sample_dggx(u: Node<Vec2<f32>>, a: Node<f32>) -> Node<Vec3<f32>> {
  let phi = val(2. * f32::PI()) * u.x();
  // NOTE: (aa-1) == (a-1)(a+1) produces better fp accuracy
  let cos_theta2 = (val(1.0) - u.y()) / (val(1.0) + (a + val(1.0)) * (a - val(1.0)) * u.y());
  let cos_theta = cos_theta2.sqrt();
  let sin_theta = (val(1.0) - cos_theta2).sqrt();
  (sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta).into()
}
