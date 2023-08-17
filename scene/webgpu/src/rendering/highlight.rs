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
      data: create_uniform(Default::default(), gpu),
    }
  }
}

impl HighLighter {
  /// We expose this function for users could use any input.
  pub fn draw_result<'a, T: 'a>(&'a self, mask: AttachmentView<T>) -> impl PassContent + 'a {
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
    scene: &SceneRenderResourceGroup,
  ) -> impl PassContent + '_
  where
    T: Iterator<Item = &'i dyn SceneRenderable>,
  {
    let mut selected_mask = attachment()
      .format(HIGH_LIGHT_MASK_TARGET_FORMAT)
      .request(ctx);

    pass("highlight-selected-mask")
      .with_color(selected_mask.write(), clear(color_same(0.)))
      .render(ctx)
      .by(scene.by_main_camera_and_self(highlight(objects)));

    self.draw_result(selected_mask.read_into())
  }
}

pub struct HighLightComposeTask<'a, T> {
  mask: AttachmentView<T>,
  lighter: &'a HighLighter,
}

impl<'a, T> ShaderPassBuilder for HighLightComposeTask<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.lighter.data);
    ctx.binding.bind(&self.mask);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
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

impl<'a, T> GraphicsShaderProvider for HighLightComposeTask<'a, T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, binding| {
      let highlighter = binding.bind_by(&self.lighter.data).expand();

      let mask = binding.bind_by(&self.mask);
      let sampler = binding.binding::<GPUSamplerView>();

      let uv = builder.query::<FragmentUv>()?;
      let size = builder.query::<RenderBufferSize>()?;

      builder.store_fragment_out(
        0,
        (
          highlighter.color.xyz(),
          edge_intensity_fn(uv, mask, sampler, highlighter.width, size) * highlighter.color.w(),
        ),
      )
    })
  }
}

#[shader_fn]
fn edge_intensity(
  uv: Node<Vec2<f32>>,
  mask: Node<ShaderTexture2D>,
  sp: Node<ShaderSampler>,
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

pub struct HighLightDrawMaskTask<T> {
  objects: Option<T>,
}

pub fn highlight<T>(objects: T) -> HighLightDrawMaskTask<T> {
  HighLightDrawMaskTask {
    objects: Some(objects),
  }
}

struct HighLightMaskDispatcher;

pub const HIGH_LIGHT_MASK_TARGET_FORMAT: TextureFormat = TextureFormat::R8Unorm;

impl ShaderHashProvider for HighLightMaskDispatcher {}
impl ShaderPassBuilder for HighLightMaskDispatcher {}

impl GraphicsShaderProvider for HighLightMaskDispatcher {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, _| {
      builder.define_out_by(channel(HIGH_LIGHT_MASK_TARGET_FORMAT));
      Ok(())
    })
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, _| builder.store_fragment_out(0, val(Vec4::one())))
  }
}

impl<'i, T> PassContentWithSceneAndCamera for HighLightDrawMaskTask<T>
where
  T: Iterator<Item = &'i dyn SceneRenderable>,
{
  fn render(
    &mut self,
    pass: &mut FrameRenderPass,
    scene: &SceneRenderResourceGroup,
    camera: &SceneCamera,
  ) {
    if let Some(objects) = self.objects.take() {
      for model in objects {
        model.render(
          pass,
          &scene.extend_bindless_resource_provider(&HighLightMaskDispatcher)
            as &dyn RenderComponentAny,
          camera,
          scene,
        )
      }
    }
  }
}
