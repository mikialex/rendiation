use crate::*;

pub struct HighLighter {
  pub data: UniformBufferDataView<HighLightData>,
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
      data: UniformBufferDataResource::create_with_source(Default::default(), &gpu.device)
        .create_default_view(),
    }
  }
}

impl HighLighter {
  /// We expose this function for users could use any input.
  pub fn draw_result<T: 'static>(&self, mask: AttachmentReadView<T>) -> impl PassContent + '_ {
    HighLightComposeTask {
      mask,
      lighter: self,
    }
    .draw_quad()
  }

  pub fn draw<'i, T>(
    &self,
    objects: T,
    ctx: &mut FrameCtx,
    scene: &Scene<WebGPUScene>,
  ) -> impl PassContent + '_
  where
    T: IntoIterator<Item = &'i dyn SceneRenderable> + Copy,
  {
    let mut selected_mask = attachment()
      .format(HIGH_LIGHT_MASK_TARGET_FORMAT)
      .request(ctx);

    pass("highlight-selected-mask")
      .with_color(selected_mask.write(), clear(color_same(0.)))
      .render(ctx)
      .by(scene.by_main_camera(highlight(objects)));

    self.draw_result(selected_mask.read_into())
  }
}

pub struct HighLightComposeTask<'a, T> {
  mask: AttachmentReadView<T>,
  lighter: &'a HighLighter,
}

impl<'a, T> ShaderPassBuilder for HighLightComposeTask<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.lighter.data, SB::Material);
    ctx.binding.bind(&self.mask, SB::Material);
    ctx.bind_immediate_sampler(&TextureSampler::default(), SB::Material);
  }
}

impl<'a, T> ShaderHashProvider for HighLightComposeTask<'a, T> {
  fn hash_pipeline(&self, _: &mut PipelineHasher) {}
}

impl<'a, T> ShaderHashProviderAny for HighLightComposeTask<'a, T> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    self.lighter.type_id().hash(hasher);
  }
}

impl<'a, T> ShaderGraphProvider for HighLightComposeTask<'a, T> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let highlighter = binding
        .uniform_by(&self.lighter.data, SB::Material)
        .expand();

      let mask = binding.uniform_by(&self.mask, SB::Material);
      let sampler = binding.uniform::<GPUSamplerView>(SB::Material);

      let uv = builder.query::<FragmentUv>()?.get();
      let size = builder.query::<RenderBufferSize>()?.get();

      builder.set_fragment_out(
        0,
        (
          highlighter.color.xyz(),
          edge_intensity(uv, mask, sampler, highlighter.width, size) * highlighter.color.w(),
        ),
      )
    })
  }
}

wgsl_fn!(
  fn edge_intensity(
    uv: vec2<f32>,
    mask: texture_2d<f32>,
    sp: sampler,
    width: f32,
    buffer_size: vec2<f32>
  ) -> f32 {
    var x_step: f32 = width / buffer_size.x;
    var y_step: f32 = width / buffer_size.y;

    var all: f32 = 0.0;
    all = all + textureSample(mask, sp, uv).x;
    all = all + textureSample(mask, sp, vec2<f32>(uv.x + x_step, uv.y)).x;
    all = all + textureSample(mask, sp, vec2<f32>(uv.x, uv.y + y_step)).x;
    all = all + textureSample(mask, sp, vec2<f32>(uv.x + x_step, uv.y+ y_step)).x;

    return (1.0 - 2.0 * abs(all / 4. - 0.5));
  }
);

pub struct HighLightDrawMaskTask<T> {
  objects: T,
}

pub fn highlight<T>(objects: T) -> HighLightDrawMaskTask<T> {
  HighLightDrawMaskTask { objects }
}

struct HighLightMaskDispatcher;

pub const HIGH_LIGHT_MASK_TARGET_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

impl ShaderHashProvider for HighLightMaskDispatcher {}
impl ShaderPassBuilder for HighLightMaskDispatcher {}

impl ShaderGraphProvider for HighLightMaskDispatcher {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, _| {
      builder.define_out_by(channel(HIGH_LIGHT_MASK_TARGET_FORMAT));
      Ok(())
    })
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, _| builder.set_fragment_out(0, consts(Vec4::one())))
  }
}

impl<'i, T> PassContentWithCamera for HighLightDrawMaskTask<T>
where
  T: IntoIterator<Item = &'i dyn SceneRenderable> + Copy,
{
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    for model in self.objects {
      model.render(pass, &HighLightMaskDispatcher, camera)
    }
  }
}
