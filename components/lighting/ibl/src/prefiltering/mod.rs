use rendiation_shader_library::sampling::{hammersley_2d_fn, sample_hemisphere_cos_fn};

use crate::*;

// pub struct PrefilteredCubeMapPair {
//   diffuse: GPUTextureCube,
//   specular: GPUTextureCube,
// }

// pub fn prefilter(cube: GPUTextureCube) -> PrefilteredCubeMapPair {
//   todo!()
// }

pub fn prefilter_diffuse(
  env: HandleNode<ShaderTextureCube>,
  sampler: HandleNode<ShaderSampler>,
  normal: Node<Vec3<f32>>,
  sampler_count: Node<u32>,
) -> Node<Vec3<f32>> {
  let tbn = tbn(normal);
  sampler_count
    .into_shader_iter()
    .map(|index| {
      let random = hammersley_2d_fn(index, sampler_count);
      let light = tbn * sample_hemisphere_cos_fn(random);
      let n_dot_l = normal.dot(light).max(0.);
      n_dot_l.greater_than(0.).select(
        env.sample_level(sampler, light, 0.).xyz(),
        val(Vec3::zero()),
      )
    })
    .sum()
    / sampler_count.into_f32().splat()
}

pub fn prefilter_specular(
  env: HandleNode<ShaderTextureCube>,
  sampler: HandleNode<ShaderSampler>,
  normal: Node<Vec3<f32>>,
  resolution: Node<f32>,
  roughness: Node<f32>,
  sampler_count: Node<u32>,
) -> Node<Vec3<f32>> {
  let tbn = tbn(normal);
  let roughness2 = roughness * roughness;

  let result = sampler_count
    .into_shader_iter()
    .map(|index| {
      let random = hammersley_2d_fn(index, sampler_count);
      let half = tbn * hemisphere_importance_sample_dggx(random, roughness2);
      let n_dot_h = normal.dot(half);
      let light = (val(2.) * n_dot_h * half - normal).normalize();
      let n_dot_l = normal.dot(light).max(0.);

      n_dot_l.greater_than(0.).select_branched(
        || {
          let pdf = d_ggx(n_dot_h, roughness2) / val(4.) + val(0.0001);
          // solid angle by this sample
          let omega_s = val(1.0) / (sampler_count.into_f32() * pdf);
          // solid angle covered by one pixel
          let omega_p = val(4. * f32::PI()) / (val(6.0) * resolution * resolution);
          let mip_level = (val(0.5) * (omega_s / omega_p).log2() + val(1.)).max(0.);

          let sample = env.sample_level(sampler, light, mip_level).xyz() * n_dot_l;
          vec4_node((sample, n_dot_l))
        },
        || val(Vec4::zero()),
      )
    })
    .sum();

  result.xyz() / result.w().splat()
}

// todo reuse
/// Microfacet Models for Refraction through Rough Surfaces - equation (33)
/// http://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html
fn d_ggx(n_o_h: Node<f32>, roughness4: Node<f32>) -> Node<f32> {
  let d = (n_o_h * roughness4 - n_o_h) * n_o_h + val(1.0);
  roughness4 / (val(f32::PI()) * d * d)
}

/// https://graphics.pixar.com/library/OrthonormalB/paper.pdf
fn tbn(normal: Node<Vec3<f32>>) -> Node<Mat3<f32>> {
  let sign = normal.z().less_than(0.).select(val(-1.), val(1.));
  let a = val(-1.) / (sign + normal.z());
  let b = normal.x() * normal.y() * a;
  let tangent = vec3_node((
    val(1.) + sign * normal.x() * normal.y() * a,
    sign * b,
    -sign * normal.x(),
  ));
  let bi_tangent = vec3_node((b, sign + normal.y() * normal.y() * a, -normal.y()));
  (tangent.normalize(), bi_tangent.normalize(), normal).into()
}
