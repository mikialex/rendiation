#![allow(clippy::too_many_arguments)]

use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct LTCRectLight {
  /// pre calculated vertex in world space.
  pub p1: Vec3<f32>,
  pub p2: Vec3<f32>,
  pub p3: Vec3<f32>,
  pub p4: Vec3<f32>,
  pub double_side: Bool,
}

only_fragment!(LtcLUT1, ShaderHandlePtr<ShaderTexture2D>);
only_fragment!(LtcLUT2, ShaderHandlePtr<ShaderTexture2D>);

const LUT_SIZE: usize = 64;

pub fn ltc_light_eval(
  light: UniformNode<LTCRectLight>,
  diffuse_color: Node<Vec3<f32>>,
  specular_color: Node<Vec3<f32>>,
  roughness: Node<f32>,
  light_color: Node<Vec3<f32>>,
  position: Node<Vec3<f32>>,
  normal: Node<Vec3<f32>>,
  view: Node<Vec3<f32>>,
  ltc_1: HandleNode<ShaderTexture2D>,
  ltc_2: HandleNode<ShaderTexture2D>,
  sampler: HandleNode<ShaderSampler>,
) -> Node<Vec3<f32>> {
  let light = light.load();

  let n_dot_v = normal.dot(view).saturate();

  let uv: Node<Vec2<_>> = (roughness, (val(1.0) - n_dot_v).sqrt()).into();
  let offset = val(0.5 / LUT_SIZE as f32).splat(); // make sure we sample pixel center.
  let uv = uv + offset;

  let t1 = ltc_1.sample(sampler, uv);
  let t2 = ltc_2.sample(sampler, uv);

  let min_v = (
    (t1.x(), val(0.), t1.y()).into(),
    (val(0.), val(1.), val(0.)).into(),
    (t1.z(), val(0.), t1.w()).into(),
  )
    .into();

  let mut spec = ltc_evaluate(normal, view, position, min_v, light, ltc_2, sampler);

  // BRDF shadowing and Fresnel
  spec *= diffuse_color * t2.x() + (val(1.0).splat() - diffuse_color) * t2.y();

  let all_one = (val(1.).splat(), val(1.).splat(), val(1.).splat()).into();

  let diff = ltc_evaluate(normal, view, position, all_one, light, ltc_2, sampler);

  (diff * diffuse_color + specular_color) * light_color
}

#[shader_fn]
pub fn ltc_evaluate(
  n: Node<Vec3<f32>>,
  v: Node<Vec3<f32>>,
  p: Node<Vec3<f32>>,
  min_v: Node<Mat3<f32>>,
  light: Node<LTCRectLight>,
  ltc_2: HandleNode<ShaderTexture2D>,
  sampler: HandleNode<ShaderSampler>,
) -> Node<Vec3<f32>> {
  let l = light.expand();
  // construct orthonormal basis around N
  let t1 = v - n * v.dot(n).splat::<Vec3<_>>();
  let t2 = n.cross(t1);

  // rotate area light in (T1, T2, N) basis
  let m: Node<Mat3<_>> = (t1, t2, n).into();
  let min_v = min_v * m.transpose();

  // polygon
  let l1 = min_v * (l.p1 - p);
  let l2 = min_v * (l.p2 - p);
  let l3 = min_v * (l.p3 - p);
  let l4 = min_v * (l.p4 - p);

  let l1 = l1.normalize();
  let l2 = l2.normalize();
  let l3 = l3.normalize();
  let l4 = l4.normalize();

  let dir = l.p1 - p;
  let light_normal = (l.p2 - l.p1).cross(l.p4 - l.p1);
  let behind = dir.dot(light_normal).less_than(0.);

  behind.not().or(l.double_side).select_branched(
    || {
      let mut v_sum = integrate_edge_vec(l1, l2);
      v_sum += integrate_edge_vec(l2, l3);
      v_sum += integrate_edge_vec(l3, l4);
      v_sum += integrate_edge_vec(l4, l1);

      let len = v_sum.length();
      let z = v_sum.z() / len;
      let z = z * behind.select(-1., 1.);

      let uv: Node<Vec2<_>> = (z * val(0.5) + val(0.5), len).into();
      let offset = val(0.5 / 64.).splat(); // make sure we sample pixel center.
      let scale = ltc_2.sample(sampler, uv + offset).z();

      (len * scale).splat()
    },
    || val(0.).splat(),
  )
}

fn integrate_edge_vec(v1: Node<Vec3<f32>>, v2: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  let x = v1.dot(v2);

  let y = x.abs();

  let a = val(0.8543985) + (val(0.4965155) + val(0.0145206) * y) * y;
  let b = val(3.417594) + (val(4.1616724) + y) * y;
  let v = a / b;

  let vv = (val(1.0) - x * x).max(1e-7).inverse_sqrt() * val(0.5) - v;

  let theta_sin_theta = x.greater_than(0.0).select(v, vv);

  v1.cross(v2) * theta_sin_theta
}
