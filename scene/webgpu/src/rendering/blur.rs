use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct LinearBlurConfig {
  pub direction: Vec2<f32>,
}

/// we separate this struct because weights data is decoupled with the blur direction
pub struct ShaderSamplingWeights {
  /// we max support 32 weight, but maybe not used them all.
  /// this array is just used as a fixed size container.
  pub weights: UniformBufferDataView<Shader140Array<Vec4<f32>, 32>>,
  /// the actually sample count we used.
  pub weight_count: UniformBufferDataView<u32>,
}

pub fn gaussian(kernel_radius: usize) -> (Shader140Array<Vec4<f32>, 32>, u32) {
  let size = 2. * kernel_radius as f32 + 1.;
  let sigma = (size + 1.) / 6.;
  let two_sigma_square = 2.0 * sigma * sigma;
  let sigma_root = (two_sigma_square * f32::PI()).sqrt();

  let mut weights: Vec<Vec4<f32>> = Vec::new();
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

  let weights = vec![Vec4::<f32>::one() * 0.1; 32].try_into().unwrap();
  let weight_count = 32;
  (weights, weight_count)
}

pub struct LinearBlurTask<'a, T> {
  input: AttachmentView<T>,
  config: &'a UniformBufferDataView<LinearBlurConfig>,
  weights: &'a ShaderSamplingWeights,
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
      let weights = binding.uniform_by(&self.weights.weights, SB::Material);
      let weight_count = binding.uniform_by(&self.weights.weight_count, SB::Material);

      let input = binding.uniform_by(&self.input, SB::Material);
      let sampler = binding.uniform::<GPUSamplerView>(SB::Material);

      let uv = builder.query::<FragmentUv>()?;
      let size = builder.query::<TexelSize>()?;

      let sum = consts(Vec4::zero()).mutable();

      let iter = ClampedShaderIter {
        inner: weights,
        count: weight_count,
      };

      let sample_offset = size * config.direction;

      for_by(iter, |_, weight, i| {
        sum.set(sum.get() + weight * input.sample(sampler, uv + i.as_f32() * sample_offset))
      });

      builder.set_fragment_out(0, sum.get())
    })
  }
}
impl<'a, T> ShaderPassBuilder for LinearBlurTask<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.config, SB::Material);
    ctx.binding.bind(&self.weights.weights, SB::Material);
    ctx.binding.bind(&self.weights.weight_count, SB::Material);
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
  weights: ShaderSamplingWeights,
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
    let x = create_uniform(x, gpu);
    let y = create_uniform(y, gpu);

    let (weights, count) = gaussian(32);
    let weights = create_uniform(weights, gpu);
    let weight_count = create_uniform(count, gpu);

    Self {
      x,
      y,
      weights: ShaderSamplingWeights {
        weights,
        weight_count,
      },
    }
  }
}

pub fn draw_linear_blur<'a, T: AsRef<Attachment> + 'a>(
  config: &'a UniformBufferDataView<LinearBlurConfig>,
  weights: &'a ShaderSamplingWeights,
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
