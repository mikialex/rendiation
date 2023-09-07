#![allow(clippy::too_many_arguments)]
#![allow(non_snake_case)]

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
  pub intensity: Vec3<f32>,
  pub double_side: Bool,
  pub is_disk: Bool,
}

only_fragment!(LtcLUT1, ShaderHandlePtr<ShaderTexture2D>);
only_fragment!(LtcLUT2, ShaderHandlePtr<ShaderTexture2D>);

const LUT_SIZE: f32 = 64.;
const LUT_SCALE: f32 = (LUT_SIZE - 1.) / LUT_SIZE;
const LUT_BIAS: f32 = 0.5 / LUT_SIZE;

pub fn ltc_light_eval(
  light: UniformNode<LTCRectLight>,
  diffuse_color: Node<Vec3<f32>>,
  specular_color: Node<Vec3<f32>>,
  roughness: Node<f32>,
  position: Node<Vec3<f32>>,
  normal: Node<Vec3<f32>>,
  view: Node<Vec3<f32>>,
  ltc_1: HandleNode<ShaderTexture2D>,
  ltc_2: HandleNode<ShaderTexture2D>,
  sampler: HandleNode<ShaderSampler>,
) -> (Node<Vec3<f32>>, Node<Vec3<f32>>) {
  let light = light.load();

  let n_dot_v = normal.dot(view).saturate();

  let uv: Node<Vec2<_>> = (roughness, (val(1.0) - n_dot_v).sqrt()).into();
  let uv = uv * val(LUT_SCALE) + val(LUT_BIAS).splat();

  let t1 = ltc_1.sample(sampler, uv);
  let t2 = ltc_2.sample(sampler, uv);

  let min_v = (
    (t1.x(), val(0.), t1.y()).into(),
    (val(0.), val(1.), val(0.)).into(),
    (t1.z(), val(0.), t1.w()).into(),
  )
    .into();

  let disk = light.expand().is_disk;
  let light_color = LTCRectLight::intensity(light);

  let mut spec = disk.select_branched(
    || ltc_evaluate_disk_fn(normal, view, position, min_v, light, ltc_2, sampler),
    || ltc_evaluate_rect_fn(normal, view, position, min_v, light, ltc_2, sampler),
  );

  // BRDF shadowing and Fresnel
  spec *= specular_color * t2.x() + (val(1.0).splat() - diffuse_color) * t2.y();

  let identity = (
    // todo useless?
    val(vec3(1., 0., 0.)),
    val(vec3(0., 1., 0.)),
    val(vec3(0., 0., 1.)),
  )
    .into();

  let diff = disk.select_branched(
    || ltc_evaluate_disk_fn(normal, view, position, identity, light, ltc_2, sampler),
    || ltc_evaluate_rect_fn(normal, view, position, identity, light, ltc_2, sampler),
  );

  (diff * diffuse_color * light_color, spec * light_color)
}

#[shader_fn]
pub fn ltc_evaluate_rect(
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
      let uv = uv * val(LUT_SCALE) + val(LUT_BIAS).splat();
      let scale = ltc_2.sample_level(sampler, uv, val(0.)).w();

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

#[shader_fn]
pub fn ltc_evaluate_disk(
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
  let t1 = t1.normalize();
  let t2 = n.cross(t1);

  // rotate area light in (T1, T2, N) basis
  let m: Node<Mat3<_>> = (t1, t2, n).into();
  let base = min_v * m.transpose();

  // polygon
  let l1 = base * (l.p1 - p);
  let l2 = base * (l.p2 - p);
  let l3 = base * (l.p3 - p);

  // init ellipse
  let C = val(0.5) * (l1 + l3);
  let V1 = val(0.5) * (l2 - l3);
  let V2 = val(0.5) * (l2 - l1);

  let C = min_v * C;
  let V1 = min_v * V1;
  let V2 = min_v * V2;

  let behind = V1.cross(V2).dot(C).less_than(0.);

  behind.not().or(l.double_side).select_branched(
    || {
      // compute eigenvectors of ellipse
      let a = val(0.).make_local_var();
      let b = val(0.).make_local_var();
      let d11 = V1.dot(V1);
      let d22 = V2.dot(V2);
      let d12 = V1.dot(V2);

      let V1 = V1.make_local_var();
      let V2 = V2.make_local_var();

      if_by(
        (d12.abs() / (d11 * d22).sqrt()).greater_than(0.0001),
        || {
          let tr = d11 + d22;
          let det = -d12 * d12 + d11 * d22;

          // use sqrt matrix to solve for eigenvalues
          let det = (det).sqrt();
          let u = val(0.5) * (tr - val(2.0) * det).sqrt();
          let v = val(0.5) * (tr + val(2.0) * det).sqrt();
          let e_max = (u + v) * (u + v);
          let e_min = (u - v) * (u - v);

          let v1_ = val(Vec3::zero()).make_local_var();
          let v2_ = val(Vec3::zero()).make_local_var();

          if_by(d11.greater_than(d22), || {
            v1_.store(d12 * V1.load() + (e_max - d11) * V2.load());
            v2_.store(d12 * V1.load() + (e_min - d11) * V2.load());
          })
          .else_by(|| {
            v1_.store(d12 * V2.load() + (e_max - d22) * V1.load());
            v2_.store(d12 * V2.load() + (e_min - d22) * V1.load());
          });

          a.store(val(1.0) / e_max);
          b.store(val(1.0) / e_min);
          V1.store(v1_.load().normalize());
          V2.store(v2_.load().normalize());
        },
      )
      .else_by(|| {
        a.store(val(1.0) / V1.load().dot(V1.load()));
        b.store(val(1.0) / V2.load().dot(V2.load()));
        V1.store(V1.load() * a.load().sqrt());
        V2.store(V2.load() * b.load().sqrt());
      });

      let V1 = V1.load();
      let V2 = V2.load();

      let V3 = V1.cross(V2).make_local_var();
      if_by(C.dot(V3.load()).less_than(0.0), || {
        V3.store(-V3.load());
      });
      let V3 = V3.load();

      let L = V3.dot(C);
      let x0 = V1.dot(C) / L;
      let y0 = V2.dot(C) / L;

      let a = a.load() * (L * L);
      let b = b.load() * (L * L);

      let c0 = a * b;
      let c1 = a * b * (val(1.0) + x0 * x0 + y0 * y0) - a - b;
      let c2 = val(1.0) - a * (val(1.0) + x0 * x0) - b * (val(1.0) + y0 * y0);
      let c3 = val(1.0);

      let roots = solve_cubic_fn((c0, c1, c2, c3).into());
      let e1 = roots.x();
      let e2 = roots.y();
      let e3 = roots.z();

      let avg_dir: Node<Vec3<_>> = (a * x0 / (a - e2), b * y0 / (b - e2), val(1.0)).into();

      let rotate: Node<Mat3<_>> = (V1, V2, V3).into();

      let avg_dir = rotate * avg_dir;
      let avg_dir = avg_dir.normalize();

      let l_1 = (-e2 / e3).sqrt();
      let l_2 = (-e2 / e1).sqrt();

      let form_factor =
        l_1 * l_2 * ((val(1.0) + l_1 * l_1) * (val(1.0) + l_2 * l_2)).inverse_sqrt();

      // use tabulated horizon-clipped sphere
      let uv: Node<Vec2<_>> = (avg_dir.z() * val(0.5) + val(0.5), form_factor).into();
      let uv = uv * val(LUT_SCALE) + val(LUT_BIAS).splat();
      let scale = ltc_2.sample_level(sampler, uv, val(0.)).w();

      let spec = form_factor * scale;

      spec.splat()
    },
    || val(0.).splat(),
  )
}

/// An extended version of the implementation from
/// "How to solve a cubic equation, revisited"
/// http://momentsingraphics.de/?p=105
#[shader_fn]
pub fn solve_cubic(coef: Node<Vec4<f32>>) -> Node<Vec3<f32>> {
  // Normalize the polynomial
  let coef: Node<Vec4<_>> = (coef.xyz() / coef.w().splat(), coef.w()).into();
  // Divide middle coefficients by three
  let coef: Node<Vec4<_>> = (coef.x(), coef.yz() / val(3.).splat(), coef.w()).into();

  let a = coef.w();
  let b = coef.z();
  let c = coef.y();
  let d = coef.x();

  // Compute the Hessian and the discriminant
  let delta: Node<Vec3<_>> = (
    -coef.z() * coef.z() + coef.y(),
    -coef.y() * coef.z() + coef.x(),
    Node::<Vec2<_>>::from((coef.z(), -coef.y())).dot(coef.xy()),
  )
    .into();

  let discriminant = Node::<Vec2<_>>::from((val(4.0) * delta.x(), -delta.y())).dot(delta.zy());

  // Algorithm A
  let xlc = {
    let c_a = delta.x();
    let d_a = -val(2.0) * b * delta.x() + delta.y();

    // Take the cubic root of a normalized complex number
    let theta = discriminant.sqrt().atan2(-d_a) / val(3.0);

    let x_1a = val(2.0) * (-c_a).sqrt() * theta.cos();
    let x_3a = val(2.0) * (-c_a).sqrt() * (theta + val((2.0 / 3.0) * f32::PI())).cos();

    let xl = (x_1a + x_3a).greater_than(val(2.) * b).select(x_1a, x_3a);

    vec2(xl - b, a)
  };

  // Algorithm D
  let xsc = {
    let c_d = delta.z();
    let d_d = -d * delta.y() + val(2.0) * c * delta.z();

    // Take the cubic root of a normalized complex number
    let theta = (d * discriminant.sqrt()).atan2(-d_d) / val(3.0);

    let x_1d = val(2.0) * (-c_d).sqrt() * theta.cos();
    let x_3d = val(2.0) * (-c_d).sqrt() * (theta + val((2.0 / 3.0) * f32::PI())).cos();

    let xs = (x_1d + x_3d).less_than(val(2.) * c).select(x_1d, x_3d);

    vec2(-d, xs + c)
  };

  let e = xlc.y() * xsc.y();
  let f = -xlc.x() * xsc.y() - xlc.y() * xsc.x();
  let g = xlc.x() * xsc.x();

  let xmc: Node<Vec2<_>> = (c * f - b * g, -b * f + c * e).into();

  let root: Node<Vec3<_>> = (xsc.x() / xsc.y(), xmc.x() / xmc.y(), xlc.x() / xlc.y()).into();

  let result = root.make_local_var();

  if_by(
    root
      .x()
      .less_than(root.y())
      .and(root.x().less_than(root.z())),
    || {
      let r = result.load();
      result.store((r.y(), r.x(), r.z()));
    },
  )
  .else_if(
    root
      .z()
      .less_than(root.x())
      .and(root.z().less_than(root.y())),
    || {
      let r = result.load();
      result.store((r.x(), r.z(), r.y()));
    },
  )
  .else_over();

  result.load()
}
