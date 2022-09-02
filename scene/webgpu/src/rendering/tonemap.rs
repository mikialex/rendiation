use crate::*;

pub struct ToneMap {
  ty: ToneMapType,
  exposure: UniformBufferDataView<f32>,
}

impl ToneMap {
  pub fn tonemap<'a, T: 'a>(&'a self, hdr: AttachmentView<T>) -> impl PassContent + 'a {
    ToneMapTask { hdr, config: self }.draw_quad()
  }
}

pub enum ToneMapType {
  Linear,
  Reinhard,
  Cineon,
  ACESFilmic,
}

struct ToneMapTask<'a, T> {
  hdr: AttachmentView<T>,
  config: &'a ToneMap,
}

impl<'a, T> ShaderHashProviderAny for ToneMapTask<'a, T> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    self.config.type_id().hash(hasher);
    self.hash_pipeline(hasher);
  }
}
impl<'a, T> ShaderHashProvider for ToneMapTask<'a, T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    std::mem::discriminant(&self.config.ty).hash(hasher)
  }
}
impl<'a, T> ShaderPassBuilder for ToneMapTask<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.config.exposure, SB::Material);
    ctx.binding.bind(&self.hdr, SB::Material);
    ctx.bind_immediate_sampler(&TextureSampler::default(), SB::Material);
  }
}

impl<'a, T> ShaderGraphProvider for ToneMapTask<'a, T> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let exposure = binding.uniform_by(&self.config.exposure, SB::Material);

      let hdr = binding.uniform_by(&self.hdr, SB::Material);
      let sampler = binding.uniform::<GPUSamplerView>(SB::Material);

      let uv = builder.query::<FragmentUv>()?.get();
      let input = hdr.sample(sampler, uv).xyz();
      let mapped = LinearToneMapping(input, exposure);

      builder.set_fragment_out(0, (mapped, 1.))
    })
  }
}

wgsl_fn!(
  // exposure only
  fn LinearToneMapping(color: vec3<f32>, toneMappingExposure: f32) -> vec3<f32> {
    return toneMappingExposure * color;
  }
);

wgsl_fn!(
  // source: https://www.cs.utah.edu/docs/techreports/2002/pdf/UUCS-02-001.pdf
  fn ReinhardToneMapping(color: vec3<f32>, toneMappingExposure: f32) -> vec3<f32> {
    color *= toneMappingExposure;
    return saturate(color / (vec3<f32>(1.0) + color));
  }
);

wgsl_fn!(
  // source: http://filmicworlds.com/blog/filmic-tonemapping-operators/
  fn OptimizedCineonToneMapping(color: vec3<f32>, toneMappingExposure: f32) -> vec3<f32> {
    // optimized filmic operator by Jim Hejl and Richard Burgess-Dawson
    color *= toneMappingExposure;
    color = max(vec3<f32>(0.0), color - 0.004);
    return pow(
      (color * (6.2 * color + 0.5)) / (color * (6.2 * color + 1.7) + 0.06),
      vec3<f32>(2.2),
    );
  }
);

wgsl_fn!(
  // source: https://github.com/selfshadow/ltc_code/blob/master/webgl/shaders/ltc/ltc_blit.fs
  fn RRTAndODTFit(v: vec3<f32>, toneMappingExposure: f32) -> vec3<f32> {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a / b;
  }
);

wgsl_fn!(
  // this implementation of ACES is modified to accommodate a brighter viewing environment.
  // the scale factor of 1/0.6 is subjective. see discussion in #19621.

  fn ACESFilmicToneMapping(color: vec3<f32>, toneMappingExposure: f32) -> vec3<f32> {
    // sRGB => XYZ => D65_2_D60 => AP1 => RRT_SAT
    let ACESInputMat = mat3x3<f32>(
      vec3<f32>( 0.59719, 0.07600, 0.02840 ), // transposed from source
      vec3<f32>( 0.35458, 0.90834, 0.13383 ),
      vec3<f32>( 0.04823, 0.01566, 0.83777 )
    );

    // ODT_SAT => XYZ => D60_2_D65 => sRGB
    let ACESOutputMat = mat3x3<f32>(
      vec3<f32>(  1.60475, -0.10208, -0.00327 ), // transposed from source
      vec3<f32>( -0.53108,  1.10813, -0.07276 ),
      vec3<f32>( -0.07367, -0.00605,  1.07602 )
    );

    color *= toneMappingExposure / 0.6;

    color = ACESInputMat * color;

    // Apply RRT and ODT
    color = RRTAndODTFit(color);

    color = ACESOutputMat * color;

    // Clamp to [0, 1]
    return saturate(color);
  }
);