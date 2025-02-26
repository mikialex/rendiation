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
  pub weights: UniformBufferCachedDataView<Shader140Array<Vec4<f32>, 32>>,
  /// the actually sample count we used.
  pub weight_count: UniformBufferCachedDataView<u32>,
}

/// radius: 0-16
pub fn gaussian(kernel_radius: usize) -> (Shader140Array<Vec4<f32>, 32>, u32) {
  let kernel_radius = kernel_radius.min(15) as i32;
  let size = 2. * kernel_radius as f32 + 1.;
  let sigma = (size + 1.) / 6.;
  let two_sigma_square = 2.0 * sigma * sigma;
  let sigma_root = (two_sigma_square * std::f32::consts::PI).sqrt();

  let mut weights: Vec<Vec4<f32>> = Vec::new();
  let mut total = Vec4::zero();
  for i in -kernel_radius..=kernel_radius {
    let distance = (i * i) as f32;
    let weight = (-distance / two_sigma_square).exp() / sigma_root;
    let weight = Vec4::splat(weight);
    weights.push(weight);
    total += weight;
  }
  weights.iter_mut().for_each(|w| *w = *w / total);
  let weight_count = weights.len();

  while weights.len() < 32 {
    weights.push(Default::default());
  }

  let weights = weights.try_into().unwrap();
  (weights, weight_count as u32)
}

pub struct LinearBlurTask<'a> {
  input: &'a RenderTargetView,
  config: &'a UniformBufferCachedDataView<LinearBlurConfig>,
  weights: &'a ShaderSamplingWeights,
}

impl ShaderHashProvider for LinearBlurTask<'_> {
  shader_hash_type_id! {UniformBufferCachedDataView<LinearBlurConfig>}
}

impl GraphicsShaderProvider for LinearBlurTask<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let config = binding.bind_by(self.config).load().expand();
      let weights = binding.bind_by(&self.weights.weights);
      let weight_count = binding.bind_by(&self.weights.weight_count).load();

      let input: BindingNode<_> = binding.bind_by(self.input);
      let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

      let uv = builder.query::<FragmentUv>();
      let size = builder.query::<TexelSize>();

      let sample_offset = size * config.direction;

      let sum = weights
        .into_shader_iter()
        .clamp_by(weight_count)
        .map(|(i, weight): (Node<u32>, ShaderReadonlyPtrOf<Vec4<f32>>)| {
          let weight = weight.load();
          let position = uv + (i.into_f32() - weight_count.into_f32() * val(0.5)) * sample_offset;
          weight * input.sample_zero_level(sampler, position)
        })
        .sum();

      builder.store_fragment_out(0, sum)
    })
  }
}
impl ShaderPassBuilder for LinearBlurTask<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.config);
    ctx.binding.bind(&self.weights.weights);
    ctx.binding.bind(&self.weights.weight_count);
    ctx.binding.bind(self.input);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
  }
}

pub fn draw_cross_blur<T: AsRef<Attachment>>(
  config: &CrossBlurData,
  input: RenderTargetView,
  ctx: &mut FrameCtx,
) -> RenderTargetView {
  let x_result = draw_linear_blur(&config.x, &config.weights, &input, ctx);
  draw_linear_blur(&config.y, &config.weights, &x_result, ctx)
}

pub struct CrossBlurData {
  x: UniformBufferCachedDataView<LinearBlurConfig>,
  y: UniformBufferCachedDataView<LinearBlurConfig>,
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
    let x = create_uniform_with_cache(x, gpu);
    let y = create_uniform_with_cache(y, gpu);

    let (weights, count) = gaussian(32);
    let weights = create_uniform_with_cache(weights, gpu);
    let weight_count = create_uniform_with_cache(count, gpu);

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

pub fn draw_linear_blur(
  config: &UniformBufferCachedDataView<LinearBlurConfig>,
  weights: &ShaderSamplingWeights,
  input: &RenderTargetView,
  ctx: &mut FrameCtx,
) -> RenderTargetView {
  let dst = input.create_attachment_key().request(ctx);

  let task = LinearBlurTask {
    input,
    config,
    weights,
  };

  pass("blur")
    .with_color(&dst, load())
    .render_ctx(ctx)
    .by(&mut task.draw_quad());

  dst
}
