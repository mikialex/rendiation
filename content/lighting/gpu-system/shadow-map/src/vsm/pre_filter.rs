use crate::*;

// todo, reuse cross blur in texture/gpu-process
/// one pass of the separable blur of the vsm moments, the sample positions
/// are clamped into the light region of the atlas so the blur never samples
/// the texels of other lights, the kernel is the box filter with linear
/// falloff weights used in the MJP Shadows sample
pub(super) struct VsmBlurTask<'a> {
  pub input: &'a GPU2DTextureView,
  pub config: &'a UniformBufferDataView<VsmMapProcessor>,
  /// the blur direction, (1, 0) for the horizontal pass and (0, 1) for the vertical pass
  pub direction: &'a UniformBufferDataView<Vec4<f32>>,
  pub map_info: &'a UniformBufferDataView<ShadowMapAddressInfo>,
}

impl ShaderHashProvider for VsmBlurTask<'_> {
  shader_hash_type_id! {VsmBlurTask<'static>}
}

impl GraphicsShaderProvider for VsmBlurTask<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let input = binding.bind_by(self.input);
      let config = binding.bind_by(self.config).load().expand();
      let direction = binding.bind_by(self.direction).load().xy();
      let info = binding.bind_by(self.map_info).load().expand();

      let position = builder.query::<FragmentPosition>().xy().floor();
      let filter_size = config.filter_size.clamp(val(1.), val(MAX_VSM_FILTER_SIZE));
      let radius = filter_size * val(0.5);
      let sample_radius = (radius + val(0.5)).floor().into_i32();

      // clamp the sample positions into the light region of the atlas
      let horizontal = direction.x().greater_than(val(0.));
      let region_min = horizontal.select(info.offset.x(), info.offset.y());
      let region_max = horizontal.select(
        info.offset.x() + info.size.x() - val(1.),
        info.offset.y() + info.size.y() - val(1.),
      );

      let sum = val(Vec4::<f32>::zero()).make_local_var();
      let i = (-sample_radius).make_local_var();
      loop_by(|cx| {
        let i_value = i.load();
        let weight = (radius + val(0.5) - i_value.abs().into_f32()).saturate();

        let sample_pos = position + direction * i_value.into_f32();
        let axis = horizontal.select(sample_pos.x(), sample_pos.y());
        let clamped_axis = axis.clamp(region_min, region_max);
        let x = horizontal.select(clamped_axis, sample_pos.x());
        let y = horizontal.select(sample_pos.y(), clamped_axis);
        let sample_coord = (x.into_u32(), y.into_u32()).into();
        let sample = input.load_texel(sample_coord, val(0));
        sum.store(sum.load() + sample * weight);

        let next = i_value + val(1);
        i.store(next);
        if_by(next.greater_than(sample_radius), || cx.do_break());
      });

      builder.store_fragment_out(0, sum.load() / filter_size.splat());
    });
  }
}

impl ShaderPassBuilder for VsmBlurTask<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.input);
    ctx.binding.bind(self.config);
    ctx.binding.bind(self.direction);
    ctx.binding.bind(self.map_info);
  }
}
