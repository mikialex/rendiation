use crate::*;

pub struct HighLighter {
  pub data: UniformBufferCachedDataView<HighLightData>,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct HighLightData {
  pub color: Vec4<f32>,
  pub width: u32,
}

impl Default for HighLightData {
  fn default() -> Self {
    Self {
      color: (0., 0.4, 8., 1.).into(),
      width: 32,
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
  /// This fn is public because this allows user use any mask (maybe from the cached one)
  pub fn draw_result(&self, mask: RenderTargetView, ctx: &mut FrameCtx) -> impl PassContent + '_ {
    let sdf = compute_sdf(
      ctx,
      mask
        .expect_standalone_common_texture_view()
        .clone()
        .try_into()
        .unwrap(),
      Some(self.data.get().width),
    );

    HighLightComputer {
      sdf: sdf.into(),
      lighter: self,
    }
    .draw_quad()
  }

  /// the passed in content should draw by `HighLightMaskDispatcher`
  pub fn draw(&self, ctx: &mut FrameCtx, mut content: impl PassContent) -> impl PassContent + '_ {
    let selected_mask = attachment()
      .format(HIGH_LIGHT_MASK_TARGET_FORMAT)
      .request(ctx);

    pass("highlight-selected-mask")
      .with_color(&selected_mask, clear_and_store(color_same(0.)))
      .render_ctx(ctx)
      .by(&mut content);

    self.draw_result(selected_mask, ctx)
  }
}

struct HighLightComputer<'a> {
  sdf: RenderTargetView,
  lighter: &'a HighLighter,
}

impl ShaderPassBuilder for HighLightComputer<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.lighter.data);
    ctx.binding.bind(&self.sdf);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
  }
}

impl ShaderHashProvider for HighLightComputer<'_> {
  shader_hash_type_id! {HighLighter}
}

impl GraphicsShaderProvider for HighLightComputer<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let highlighter = binding.bind_by(&self.lighter.data).load().expand();

      let sdf = binding.bind_by(&self.sdf);
      let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

      let uv = builder.query::<FragmentUv>();
      let current_coord = builder.query::<FragmentPosition>().xy().floor();

      let nearest_border_coord = sdf.sample_zero_level(sampler, uv).xy();

      let alpha = nearest_border_coord
        .equals(Vec2::splat(f32::MAX))
        .all()
        .or(nearest_border_coord.equals(current_coord).all())
        .select_branched(
          || val(0.),
          || {
            let distance = (current_coord - nearest_border_coord).length();

            val(1.) - distance / highlighter.width.into_f32()
          },
        );

      let output: Node<Vec4<f32>> = (highlighter.color.xyz(), alpha).into();

      builder.store_fragment_out(0, output)
    })
  }
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
