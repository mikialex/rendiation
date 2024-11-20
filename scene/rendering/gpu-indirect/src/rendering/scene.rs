use crate::*;

pub struct IndirectRenderSystem {
  pub model_lookup: UpdateResultToken,
  pub texture_system: TextureGPUSystemSource,
  pub camera: Box<dyn RenderImplProvider<Box<dyn CameraRenderImpl>>>,
  pub scene_model_impl: Box<dyn RenderImplProvider<Box<dyn IndirectBatchSceneModelRenderer>>>,
  // pub grouper: Box<dyn RenderImplProvider<Box<dyn IndirectSceneDrawBatchGrouper>>>,
}

pub fn build_default_gles_render_system() -> IndirectRenderSystem {
  IndirectRenderSystem {
    model_lookup: Default::default(),
    texture_system: Default::default(),
    camera: todo!(),
    scene_model_impl: Box::new(IndirectPreferredComOrderRendererProvider {
      node: Box::new(DefaultIndirectNodeRenderImplProvider::default()),
      model_impl: vec![],
    }),
  }
}

impl RenderImplProvider<Box<dyn SceneRenderer<ContentKey = SceneContentKey>>>
  for IndirectRenderSystem
{
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.texture_system.register_resource(source, cx);

    let model_lookup = global_rev_ref().watch_inv_ref::<SceneModelBelongsToScene>();
    self.model_lookup = source.register_multi_reactive_query(model_lookup);
    self.camera.register_resource(source, cx);
    self.scene_model_impl.register_resource(source, cx);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    self.texture_system.deregister_resource(source);
    self.camera.deregister_resource(source);
    self.scene_model_impl.deregister_resource(source);
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn SceneRenderer<ContentKey = SceneContentKey>> {
    Box::new(IndirectSceneRenderer {
      texture_system: self.texture_system.create_impl(res),
      camera: self.camera.create_impl(res),
      background: SceneBackgroundRenderer::new_from_global(),
      renderer: self.scene_model_impl.create_impl(res),
      // grouper: self.grouper.create_impl(res),
    })
  }
}

struct IndirectSceneRenderer {
  texture_system: GPUTextureBindingSystem,
  camera: Box<dyn CameraRenderImpl>,

  background: SceneBackgroundRenderer,

  renderer: Box<dyn IndirectBatchSceneModelRenderer>,
  // grouper: Box<dyn IndirectSceneDrawBatchGrouper>,
}

impl SceneModelRenderer for IndirectSceneRenderer {
  fn render_scene_model(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) -> Result<(), UnableToRenderSceneModelError> {
    self.renderer.render_scene_model(idx, camera, pass, cx, tex)
  }
}

// todo, impl better render models using host side immediate prepared indirect draw
impl SceneRenderer for IndirectSceneRenderer {
  type ContentKey = SceneContentKey;
  fn extract_scene_batch(
    &self,
    _scene: EntityHandle<SceneEntity>,
    _semantic: Self::ContentKey,
    _ctx: &mut FrameCtx,
  ) -> SceneModelRenderBatch {
    todo!()
  }

  fn make_scene_batch_pass_content<'a>(
    &'a self,
    batch: SceneModelRenderBatch,
    camera: EntityHandle<SceneCameraEntity>,
    pass: &'a dyn RenderComponent,
    ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a> {
    let batch = batch.get_device_batch(None).unwrap();

    let content: Vec<_> = batch
      .sub_batches
      .iter()
      .map(|batch| {
        let any_scene_model = batch.impl_select_id;
        let draw_command_builder = self
          .renderer
          .make_draw_command_builder(batch.impl_select_id)
          .unwrap();

        let provider = ctx.access_parallel_compute(|cx| {
          batch.create_indirect_draw_provider(draw_command_builder, cx)
        });

        (provider, any_scene_model)
      })
      .collect();

    Box::new(IndirectScenePassContent {
      renderer: self,
      content,
      pass,
      camera,
    })
  }

  fn init_clear(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> (Operations<rendiation_webgpu::Color>, Operations<f32>) {
    self.background.init_clear(scene)
  }

  fn get_scene_model_cx(&self) -> &GPUTextureBindingSystem {
    &self.texture_system
  }

  fn get_camera_gpu(&self) -> &dyn CameraRenderImpl {
    self.camera.as_ref()
  }
}

struct IndirectScenePassContent<'a> {
  renderer: &'a IndirectSceneRenderer,
  content: Vec<(
    Box<dyn IndirectDrawProvider>,
    EntityHandle<SceneModelEntity>,
  )>,

  pass: &'a dyn RenderComponent,
  camera: EntityHandle<SceneCameraEntity>,
}

impl<'a> PassContent for IndirectScenePassContent<'a> {
  fn render(&mut self, cx: &mut FrameRenderPass) {
    let camera = self.renderer.camera.make_component(self.camera).unwrap();
    for (content, any_scene_model) in &self.content {
      self.renderer.renderer.render_indirect_batch_models(
        content.as_ref(),
        *any_scene_model,
        &camera,
        &self.renderer.texture_system,
        &self.pass,
        &mut cx.ctx,
      );
    }
  }
}
