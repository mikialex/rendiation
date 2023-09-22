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
