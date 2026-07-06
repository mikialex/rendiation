use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct BilateralBlurConfig {
  pub direction: Vec2<f32>,
  pub depth_sigma: f32,
}

impl Default for BilateralBlurConfig {
  fn default() -> Self {
    Self {
      direction: Vec2::new(0., 0.),
      depth_sigma: 0.5,
      ..Zeroable::zeroed()
    }
  }
}

pub struct BilateralBlurTask<'a> {
  value_input: &'a RenderTargetView,
  depth: &'a RenderTargetView,
  config: &'a UniformBufferCachedDataView<BilateralBlurConfig>,
  weights: &'a ShaderSamplingWeights,
}

impl ShaderHashProvider for BilateralBlurTask<'_> {
  shader_hash_type_id! {BilateralBlurTask<'static>}
}

impl ShaderPassBuilder for BilateralBlurTask<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.value_input);
    ctx.binding.bind(self.depth);
    ctx.binding.bind(self.config);
    ctx.binding.bind(&self.weights.weights);
    ctx.binding.bind(&self.weights.weight_count);
    let depth_sampler_desc = TextureSampler::default().into_gpu();
    ctx.bind_immediate_sampler(&depth_sampler_desc); // depth_sampler (NonFiltering)
    let value_sampler_desc = TextureSampler::default().with_double_linear().into_gpu();
    ctx.bind_immediate_sampler(&value_sampler_desc); // value_sampler (Filtering, linear)
  }
}

impl GraphicsShaderProvider for BilateralBlurTask<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let value_input = binding.bind_by(self.value_input);
      let depth_tex = binding.bind_by(&DisableFiltering(self.depth));
      let config = binding.bind_by(self.config).load().expand();
      let weights = binding.bind_by(&self.weights.weights);
      let weight_count = binding.bind_by(&self.weights.weight_count).load();
      let depth_sampler = binding.bind_by(&DisableFiltering(ImmediateGPUSamplerViewBind));
      let value_sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

      let uv = builder.query::<FragmentUv>();
      let texel_size = builder.query::<TexelSize>();

      let sample_offset = texel_size * config.direction;
      let center_depth = depth_tex.sample_zero_level(depth_sampler, uv).x();

      let weight_count = weight_count.x();
      let two_sigma2 = (val(2.0) * config.depth_sigma * config.depth_sigma).max(val(0.0));
      let radius = (weight_count.into_f32() - val(1.0)) * val(0.5);

      let combined = weights
        .into_shader_iter()
        .clamp_by(weight_count)
        .map(|(i, weight): (Node<u32>, ShaderReadonlyPtrOf<Vec4<f32>>)| {
          let weight = weight.load();
          let offset = i.into_f32() - radius;
          let sample_uv = uv + offset * sample_offset;

          let depth_sample = depth_tex.sample_zero_level(depth_sampler, sample_uv).x();
          let depth_diff = depth_sample - center_depth;
          let edge_weight = (-depth_diff * depth_diff / two_sigma2).exp();

          let w = weight.x() * edge_weight;
          let value = value_input.sample_zero_level(value_sampler, sample_uv);
          (value.x() * w, value.y() * w, value.z() * w, w).into()
        })
        .sum();

      let result = (
        combined.x() / combined.w(),
        combined.y() / combined.w(),
        combined.z() / combined.w(),
        val(1.0),
      )
        .into();

      builder.store_fragment_out(0, result)
    });
  }
}

pub struct BilateralBlurData {
  x: UniformBufferCachedDataView<BilateralBlurConfig>,
  y: UniformBufferCachedDataView<BilateralBlurConfig>,
  weights: ShaderSamplingWeights,
}

impl BilateralBlurData {
  pub fn new(gpu: &GPU) -> Self {
    let x = BilateralBlurConfig {
      direction: Vec2::new(1., 0.),
      ..Default::default()
    };
    let y = BilateralBlurConfig {
      direction: Vec2::new(0., 1.),
      ..Default::default()
    };
    let x = create_uniform_with_cache(x, gpu, "x bilateral blur config");
    let y = create_uniform_with_cache(y, gpu, "y bilateral blur config");

    let (weights_data, count) = gaussian(8);
    let weights = create_uniform_with_cache(weights_data, gpu, "blur weights");
    let weight_count = create_uniform_with_cache(Vec4::splat(count), gpu, "blur weight count");

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

pub fn draw_bilateral_blur(
  config: &UniformBufferCachedDataView<BilateralBlurConfig>,
  weights: &ShaderSamplingWeights,
  value_input: &RenderTargetView,
  depth: &RenderTargetView,
  ctx: &mut FrameCtx,
) -> RenderTargetView {
  let dst = value_input.create_attachment_key().request(ctx);

  let task = BilateralBlurTask {
    value_input,
    depth,
    config,
    weights,
  };

  pass("bilateral blur")
    .with_color(&dst, store_full_frame())
    .render_ctx(ctx)
    .by(&mut task.draw_quad());

  dst
}

pub fn draw_cross_bilateral_blur(
  data: &BilateralBlurData,
  value_input: RenderTargetView,
  depth: &RenderTargetView,
  ctx: &mut FrameCtx,
) -> RenderTargetView {
  let x_result = draw_bilateral_blur(&data.x, &data.weights, &value_input, depth, ctx);
  draw_bilateral_blur(&data.y, &data.weights, &x_result, depth, ctx)
}
