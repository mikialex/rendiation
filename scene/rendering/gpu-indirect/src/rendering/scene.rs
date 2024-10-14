use crate::*;

pub struct IndirectRenderSystem {
  pub model_lookup: UpdateResultToken,
  pub texture_system: TextureGPUSystemSource,
  pub camera: Box<dyn RenderImplProvider<Box<dyn CameraRenderImpl>>>,
  pub scene_model_impl: Vec<Box<dyn RenderImplProvider<Box<dyn SceneModelRenderer>>>>,
}

impl RenderImplProvider<Box<dyn SceneRenderer>> for IndirectRenderSystem {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.texture_system.register_resource(source, cx);

    let model_lookup = global_rev_ref().watch_inv_ref::<SceneModelBelongsToScene>();
    self.model_lookup = source.register_reactive_multi_collection(model_lookup);
    self.camera.register_resource(source, cx);
    //   for imp in &mut self.scene_model_impl {
    //     imp.register_resource(source, cx);
    //   }
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> Box<dyn SceneRenderer> {
    Box::new(IndirectSceneRenderer {
      texture_system: self.texture_system.create_impl(res),
      camera: todo!(),
      background: SceneBackgroundRenderer::new_from_global(),
      renderer: todo!(),
      grouper: todo!(),
    })
  }
}

struct IndirectSceneRenderer {
  texture_system: GPUTextureBindingSystem,
  camera: Box<dyn CameraRenderImpl>,

  background: SceneBackgroundRenderer,

  renderer: Box<dyn IndirectBatchSceneModelRenderer>,

  grouper: Box<dyn IndirectSceneDrawBatchGrouper>,
}

impl SceneModelRenderer for IndirectSceneRenderer {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    camera_gpu: &'a (dyn CameraRenderImpl + 'a),
    pass: &'a (dyn RenderComponent + 'a),
    tex: &'a GPUTextureBindingSystem,
  ) -> Option<(Box<dyn RenderComponent + 'a>, DrawCommand)> {
    self
      .renderer
      .make_component(idx, camera, camera_gpu, pass, tex)
  }
}

impl SceneRenderer for IndirectSceneRenderer {
  fn make_pass_content<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    pass: &'a dyn RenderComponent,
    ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a> {
    // do gpu driven culling here in future
    Box::new(GLESScenePassContent {
      renderer: self,
      scene,
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

  fn render_reorderable_models(
    &self,
    models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
    camera: EntityHandle<SceneCameraEntity>,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) {
    self.renderer.render_reorderable_models_impl(
      models,
      camera,
      self.camera.as_ref(),
      pass,
      cx,
      tex,
    );
  }

  fn get_camera_gpu(&self) -> &dyn CameraRenderImpl {
    self.camera.as_ref()
  }
}

struct GLESScenePassContent<'a> {
  renderer: &'a IndirectSceneRenderer,
  scene: EntityHandle<SceneEntity>,
  pass: &'a dyn RenderComponent,
  camera: EntityHandle<SceneCameraEntity>,
}

impl<'a> PassContent for GLESScenePassContent<'a> {
  fn render(&mut self, cx: &mut FrameRenderPass) {
    for (indirect_batch, any_id) in self.renderer.grouper.iter_grouped_scene_model(self.scene) {
      // do indirect dispatches here
      self.renderer.renderer.render_batch_models(
        indirect_batch,
        any_id,
        self.camera,
        &self.renderer.texture_system,
        &self.pass,
        cx,
      );
    }
  }
}
