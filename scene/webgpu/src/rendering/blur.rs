use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct LinearBlurConfig {
  pub direction: Vec2<f32>,
}

/// we separate this struct because weights data is decoupled with the blur direction
#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct ShaderSamplingWeights {
  /// we max support 32 weight, but maybe not used them all.
  /// this array is just used as a fixed size container.
  pub weights: Shader140Array<f32, 32>,
  /// the actually sample count we used.
  pub weight_count: u32,
}

impl ShaderSamplingWeights {
  pub fn gaussian(kernel_radius: usize) -> Self {
    let size = 2. * kernel_radius as f32 + 1.;
    let sigma = (size + 1.) / 6.;
    let two_sigma_square = 2.0 * sigma * sigma;
    let sigma_root = (two_sigma_square * f32::PI()).sqrt();

    let mut weights: Vec<f32> = Vec::new();
    let mut total = 0.0;
    // for (let i = -kernelRadius; i <= kernelRadius; ++i) {
    //     const distance = i * i;
    //     const index = i + kernelRadius;
    //     weights[index] = Math.exp(-distance / twoSigmaSquare) / sigmaRoot;
    //     total += weights[index];
    // }
    // for (let i = 0; i < weights.length; i++) {
    //     weights[i] /= total;
    // }
    let weight_count = weights.len();

    let weights = vec![0.; 32].try_into().unwrap();
    let weight_count = 32;
    Self {
      weights,
      weight_count,
      ..Zeroable::zeroed()
    }
  }
}

pub struct LinearBlurTask<'a, T> {
  input: AttachmentView<T>,
  config: &'a UniformBufferDataView<LinearBlurConfig>,
  weights: &'a UniformBufferDataView<ShaderSamplingWeights>,
}

impl<'a, T> ShaderHashProvider for LinearBlurTask<'a, T> {}
impl<'a, T> ShaderHashProviderAny for LinearBlurTask<'a, T> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    self.config.type_id().hash(hasher);
  }
}
impl<'a, T> ShaderGraphProvider for LinearBlurTask<'a, T> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.log_result = true;
    builder.fragment(|builder, binding| {
      let config = binding.uniform_by(self.config, SB::Material).expand();
      let weights = binding.uniform_by(self.weights, SB::Material);

      let input = binding.uniform_by(&self.input, SB::Material);
      let sampler = binding.uniform::<GPUSamplerView>(SB::Material);

      let uv = builder.query::<FragmentUv>()?.get();
      let size = builder.query::<TexelSize>()?.get();

      let blurred = linear_blur(config.direction, weights, input, sampler, uv, size);

      builder.set_fragment_out(0, blurred)
    })
  }
}
impl<'a, T> ShaderPassBuilder for LinearBlurTask<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.config, SB::Material);
    ctx.binding.bind(self.weights, SB::Material);
    ctx.binding.bind(&self.input, SB::Material);
    ctx.bind_immediate_sampler(&TextureSampler::default(), SB::Material);
  }
}

pub fn draw_cross_blur<T: AsRef<Attachment>>(
  config: &CrossBlurData,
  input: AttachmentView<T>,
  ctx: &mut FrameCtx,
) -> Attachment {
  let x_result = draw_linear_blur(&config.x, &config.weights, input, ctx);
  draw_linear_blur(&config.y, &config.weights, x_result.read_into(), ctx)
}

pub struct CrossBlurData {
  x: UniformBufferDataView<LinearBlurConfig>,
  y: UniformBufferDataView<LinearBlurConfig>,
  weights: UniformBufferDataView<ShaderSamplingWeights>,
}

impl CrossBlurData {
  pub fn new(gpu: &GPU) -> Self {
    let x = LinearBlurConfig {
      direction: Vec2::new(1., 0.),
      ..Zeroable::zeroed()
    };
    let y = LinearBlurConfig {
      direction: Vec2::new(0., 1.),
      ..Zeroable::zeroed()
    };
    let x = UniformBufferDataResource::create_with_source(x, &gpu.device).create_default_view();
    let y = UniformBufferDataResource::create_with_source(y, &gpu.device).create_default_view();

    let weights = ShaderSamplingWeights::gaussian(32);
    let weights =
      UniformBufferDataResource::create_with_source(weights, &gpu.device).create_default_view();

    Self { x, y, weights }
  }
}

pub fn draw_linear_blur<'a, T: AsRef<Attachment> + 'a>(
  config: &'a UniformBufferDataView<LinearBlurConfig>,
  weights: &'a UniformBufferDataView<ShaderSamplingWeights>,
  input: AttachmentView<T>,
  ctx: &mut FrameCtx,
) -> Attachment {
  let mut dst = input.resource().as_ref().des().clone().request(ctx);

  let task: LinearBlurTask<'a, T> = LinearBlurTask {
    input,
    config,
    weights,
  };

  pass("blur")
    .with_color(dst.write(), load())
    .render(ctx)
    .by(task.draw_quad());

  dst
}

wgsl_fn!(
  fn lin_space(w0: f32, d0: vec4<f32>, w1: f32, d1: vec4<f32>) -> f32 {
    return (w0 * d0 + w1 * d1);
  }
);

wgsl_fn!(
  fn linear_blur(
    direction: vec2<f32>,
    weights: ShaderSamplingWeights,
    texture: texture_2d<f32>,
    sp: sampler,
    uv: vec2<f32>,
    texel_size: vec2<f32>,
  ) -> vec4<f32> {
    let sample_offset = texel_size * direction;
    var sum: vec4<f32>;
    for (var i: i32 = 2; i < weights.weight_count; i++) {
        let samples = textureSample(texture, sp, uv + f32(i) * sample_offset);
        sum = lin_space(1.0, sum, weights.weights[i].inner, samples);
    }
    return sum;
  }
);
