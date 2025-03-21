use crate::*;

pub struct HighLighter {
  pub data: UniformBufferCachedDataView<HighLightData>,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct HighLightData {
  pub color: Vec4<f32>,
  pub width: f32,
}

impl Default for HighLightData {
  fn default() -> Self {
    Self {
      color: (0., 0.4, 8., 1.).into(),
      width: 2.,
      ..Zeroable::zeroed()
    }
  }
}

impl HighLighter {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      data: create_uniform_with_cache(Default::default(), gpu),
    }
  }
}

impl HighLighter {
  /// We expose this function for users could use any input.
  pub fn draw_result(&self, mask: RenderTargetView) -> impl PassContent + '_ {
    HighLightComposeTask {
      mask,
      lighter: self,
    }
    .draw_quad()
  }

  /// scene should masked by `HighLightMaskDispatcher`
  pub fn draw(&self, ctx: &mut FrameCtx, mut content: impl PassContent) -> impl PassContent + '_ {
    let selected_mask = attachment()
      .format(HIGH_LIGHT_MASK_TARGET_FORMAT)
      .request(ctx);

    pass("highlight-selected-mask")
      .with_color(&selected_mask, clear(color_same(0.)))
      .render_ctx(ctx)
      .by(&mut content);

    self.draw_result(selected_mask)
  }
}

pub struct HighLightComposeTask<'a> {
  mask: RenderTargetView,
  lighter: &'a HighLighter,
}

impl ShaderPassBuilder for HighLightComposeTask<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.lighter.data);
    ctx.binding.bind(&self.mask);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
  }
}

impl ShaderHashProvider for HighLightComposeTask<'_> {
  shader_hash_type_id! {HighLighter}
}

impl GraphicsShaderProvider for HighLightComposeTask<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let highlighter = binding.bind_by(&self.lighter.data).load().expand();

      let mask = binding.bind_by(&self.mask);
      let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

      let uv = builder.query::<FragmentUv>();
      let size = builder.query::<RenderBufferSize>();

      let alpha =
        edge_intensity_fn(uv, mask, sampler, highlighter.width, size) * highlighter.color.w();
      let output: Node<Vec4<f32>> = (highlighter.color.xyz(), alpha).into();

      builder.store_fragment_out(0, output)
    })
  }
}

#[shader_fn]
fn edge_intensity(
  uv: Node<Vec2<f32>>,
  mask: BindingNode<ShaderTexture2D>,
  sp: BindingNode<ShaderSampler>,
  width: Node<f32>,
  buffer_size: Node<Vec2<f32>>,
) -> Node<f32> {
  let x_step = width / buffer_size.x();
  let y_step = width / buffer_size.y();

  let mut all = val(0.0);
  all += mask.sample(sp, uv).x();
  all += mask.sample(sp, (uv.x() + x_step, uv.y())).x();
  all += mask.sample(sp, (uv.x(), uv.y() + y_step)).x();
  all += mask.sample(sp, (uv.x() + x_step, uv.y() + y_step)).x();

  val(1.0) - val(2.0) * (all / val(4.) - val(0.5)).abs()
}

pub struct HighLightMaskDispatcher;

pub const HIGH_LIGHT_MASK_TARGET_FORMAT: TextureFormat = TextureFormat::R8Unorm;

impl ShaderHashProvider for HighLightMaskDispatcher {
  shader_hash_type_id! {}
}
impl ShaderPassBuilder for HighLightMaskDispatcher {}

impl GraphicsShaderProvider for HighLightMaskDispatcher {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      builder.frag_output.first_mut().unwrap().states =
        channel(HIGH_LIGHT_MASK_TARGET_FORMAT).into();
    })
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      builder
        .frag_output
        .iter_mut()
        .for_each(|p| p.states.blend = None);
      builder.store_fragment_out(0, val(1.0));
    })
  }
}
