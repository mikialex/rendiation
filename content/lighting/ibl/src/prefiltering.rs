use rendiation_shader_library::sampling::{hammersley_2d_fn, sample_hemisphere_cos_fn, tbn_fn};

use crate::*;

pub fn prefilter_diffuse(
  env: HandleNode<ShaderTextureCube>,
  sampler: HandleNode<ShaderSampler>,
  normal: Node<Vec3<f32>>,
  sampler_count: Node<u32>,
) -> Node<Vec3<f32>> {
  let tbn = tbn_fn(normal);
  sampler_count
    .into_shader_iter()
    .map(|index| {
      let random = hammersley_2d_fn(index, sampler_count);
      let light = tbn * sample_hemisphere_cos_fn(random);
      let n_dot_l = normal.dot(light).max(0.);
      n_dot_l.greater_than(0.).select(
        env.sample_zero_level(sampler, light).xyz(),
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
  let tbn = tbn_fn(normal);
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

          let sample = env
            .build_sample_call(sampler, light)
            .with_level(mip_level)
            .sample()
            .xyz()
            * n_dot_l;
          vec4_node((sample, n_dot_l))
        },
        || val(Vec4::zero()),
      )
    })
    .sum();

  result.xyz() / result.w().splat()
}

struct BrdfLUTGenerator;
impl ShaderPassBuilder for BrdfLUTGenerator {}
impl ShaderHashProvider for BrdfLUTGenerator {
  shader_hash_type_id! {}
}
impl GraphicsShaderProvider for BrdfLUTGenerator {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let sample_count = val(32);
      let uv = builder.query::<FragmentUv>();
      let result = integrate_brdf(uv.x(), uv.y(), sample_count);
      builder.store_fragment_out(0, (result, val(1.), val(1.)))
    })
  }
}

pub fn generate_brdf_lut(ctx: &mut FrameCtx, target: GPU2DTextureView) {
  pass("brdf lut generate")
    .with_color(target, load())
    .render_ctx(ctx)
    .by(&mut BrdfLUTGenerator.draw_quad());
}

// pub struct PrefilteredCubeMapPair {
//   diffuse: GPUTextureCube,
//   specular: GPUTextureCube,
// }

// pub fn prefilter(cube: GPUTextureCube) -> PrefilteredCubeMapPair {
//   todo!()
// }
