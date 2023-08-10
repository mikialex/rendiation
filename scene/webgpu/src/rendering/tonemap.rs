use crate::*;

pub struct ToneMap {
  ty: ToneMapType,
  exposure: UniformBufferDataView<f32>,
}

impl ToneMap {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      ty: ToneMapType::Linear,
      exposure: create_uniform(1., gpu),
    }
  }
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
impl ShaderHashProvider for ToneMap {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    std::mem::discriminant(&self.ty).hash(hasher)
  }
}
impl ShaderPassBuilder for ToneMap {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.exposure);
  }
}
impl GraphicsShaderProvider for ToneMap {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let exposure = binding.bind_by(&self.exposure);
      let hdr = builder.query::<HDRLightResult>()?;

      let mapped = match self.ty {
        ToneMapType::Linear => linear_tone_mapping(hdr, exposure),
        ToneMapType::Reinhard => reinhard_tone_mapping(hdr, exposure),
        ToneMapType::Cineon => optimized_cineon_tone_mapping(hdr, exposure),
        ToneMapType::ACESFilmic => ACESFilmicToneMapping(hdr, exposure),
      };

      builder.register::<LDRLightResult>(mapped);
      Ok(())
    })
  }
}

#[shadergraph_fn]
fn linear_tone_mapping(color: Node<Vec3<f32>>, exposure: Node<f32>) -> Node<Vec3<f32>> {
  exposure * color
}

/// source: https://www.cs.utah.edu/docs/techreports/2002/pdf/UUCS-02-001.pdf
#[shadergraph_fn]
fn reinhard_tone_mapping(color: Node<Vec3<f32>>, exposure: Node<f32>) -> Node<Vec3<f32>> {
  let color = exposure * color;
  let mapped = color / (val(Vec3::one()) + color);
  mapped.saturate()
}

// val vec3 splat
fn val_v3s(f: f32) -> Node<Vec3<f32>> {
  val(Vec3::splat(f))
}

/// optimized filmic operator by Jim Hejl and Richard Burgess-Dawson
/// source: http://filmicworlds.com/blog/filmic-tonemapping-operators/
fn optimized_cineon_tone_mapping(color: Node<Vec3<f32>>, exposure: Node<f32>) -> Node<Vec3<f32>> {
  let color = exposure * color;
  let color = (color - val_v3s(0.004)).max(val(Vec3::zero()));
  let color = (color * (val(6.2) * color + val_v3s(0.5)))
    / (color * (val(6.2) * color + val_v3s(1.7)) + val_v3s(0.06));
  color.pow(val(2.2))
}

wgsl_fn!(
  // source: https://github.com/selfshadow/ltc_code/blob/master/webgl/shaders/ltc/ltc_blit.fs
  fn RRTAndODTFit(v: vec3<f32>, toneMappingExposure: f32) -> vec3<f32> {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a / b;
  }
);

// source: https://github.com/selfshadow/ltc_code/blob/master/webgl/shaders/ltc/ltc_blit.fs
fn rrt_and_odt_fit(v: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  let a = v * (v + val_v3s(0.0245786)) - val_v3s(0.000090537);
  let b = v * (val(0.983729) * v + val_v3s(0.432951)) + val_v3s(0.238081);
  a / b
}

wgsl_fn!(
  // this implementation of ACES is modified to accommodate a brighter viewing environment.
  // the scale factor of 1/0.6 is subjective. see discussion in #19621.
  fn ACESFilmicToneMapping(c: vec3<f32>, toneMappingExposure: f32) -> vec3<f32> {
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

    var color = c;

    color *= toneMappingExposure / 0.6;

    color = ACESInputMat * color;

    // Apply RRT and ODT
    color = RRTAndODTFit(color, toneMappingExposure);

    color = ACESOutputMat * color;

    // Clamp to [0, 1]
    return saturate(color);
  }
);

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
    self.config.hash_pipeline(hasher)
  }
}
impl<'a, T> ShaderPassBuilder for ToneMapTask<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.hdr);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
    self.config.setup_pass(ctx)
  }
}

impl<'a, T> GraphicsShaderProvider for ToneMapTask<'a, T> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let hdr = binding.bind_by(&self.hdr);
      let sampler = binding.binding::<GPUSamplerView>();

      let uv = builder.query::<FragmentUv>()?;
      let hdr = hdr.sample(sampler, uv).xyz();

      builder.register::<HDRLightResult>(hdr);
      Ok(())
    })?;

    self.config.build(builder)?;

    builder.fragment(|builder, _| {
      let ldr = builder.query::<LDRLightResult>()?;
      builder.set_fragment_out(0, (ldr, val(1.)))
    })
  }
}
