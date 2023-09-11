use rendiation_shader_library::sampling::hammersley_2d_fn;

use crate::*;

pub fn integrate_brdf(
  roughness: Node<f32>, // perceptual roughness
  n_dot_v: Node<f32>,
  sample_count: Node<u32>,
) -> Node<Vec2<f32>> {
  let roughness2 = roughness * roughness;
  let view: Node<Vec3<_>> = ((val(1.) - n_dot_v * n_dot_v).sqrt(), val(0.), n_dot_v).into();

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

      let r: Node<Vec2<f32>> = ((val(1.) - fc) * g_vis, fc * g_vis).into();
      r
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
