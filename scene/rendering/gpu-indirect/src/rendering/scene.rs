use crate::*;

pub struct IndirectRenderSystem {
  pub model_lookup: UpdateResultToken,
  pub texture_system: TextureGPUSystemSource,
  pub camera: Box<dyn RenderImplProvider<Box<dyn CameraRenderImpl>>>,
  pub scene_model_impl: Vec<Box<dyn RenderImplProvider<Box<dyn IndirectBatchSceneModelRenderer>>>>,
  pub grouper: Box<dyn RenderImplProvider<Box<dyn IndirectSceneDrawBatchGrouper>>>,
}

impl RenderImplProvider<Box<dyn SceneRenderer<ContentKey = SceneContentKey>>>
  for IndirectRenderSystem
{
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.texture_system.register_resource(source, cx);

    let model_lookup = global_rev_ref().watch_inv_ref::<SceneModelBelongsToScene>();
    self.model_lookup = source.register_multi_reactive_query(model_lookup);
    self.camera.register_resource(source, cx);
    for imp in &mut self.scene_model_impl {
      imp.register_resource(source, cx);
    }
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn SceneRenderer<ContentKey = SceneContentKey>> {
    Box::new(IndirectSceneRenderer {
      texture_system: self.texture_system.create_impl(res),
      camera: self.camera.create_impl(res),
      background: SceneBackgroundRenderer::new_from_global(),
      renderer: self
        .scene_model_impl
        .iter()
        .map(|imp| imp.create_impl(res))
        .collect(),
      grouper: self.grouper.create_impl(res),
    })
  }
}

struct IndirectSceneRenderer {
  texture_system: GPUTextureBindingSystem,
  camera: Box<dyn CameraRenderImpl>,

  background: SceneBackgroundRenderer,

  renderer: Vec<Box<dyn IndirectBatchSceneModelRenderer>>,

  grouper: Box<dyn IndirectSceneDrawBatchGrouper>,
}

impl SceneModelRenderer for IndirectSceneRenderer {
  fn render_scene_model(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) -> Option<()> {
    for r in &self.renderer {
      if r.render_scene_model(idx, camera, pass, cx, tex).is_some() {
        return Some(());
      }
    }
    None
  }
}

impl SceneRenderer for IndirectSceneRenderer {
  type ContentKey = SceneContentKey;
  fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: Self::ContentKey,
    _ctx: &mut FrameCtx,
  ) -> SceneModelRenderBatch {
    todo!()
  }

  fn make_scene_batch_pass_content(
    &self,
    batch: SceneModelRenderBatch,
    camera: EntityHandle<SceneCameraEntity>,
    pass: &dyn RenderComponent,
    _ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent> {
    todo!()
    // Box::new(IndirectScenePassContent {
    //   renderer: self,
    //   scene,
    //   pass,
    //   camera: semantic.camera,
    // })
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
  scene: EntityHandle<SceneEntity>,
  pass: &'a dyn RenderComponent,
  camera: EntityHandle<SceneCameraEntity>,
}

impl<'a> PassContent for IndirectScenePassContent<'a> {
  fn render(&mut self, cx: &mut FrameRenderPass) {
    let camera = self.renderer.camera.make_component(self.camera).unwrap();
    for (indirect_batch, any_id) in self.renderer.grouper.iter_grouped_scene_model(self.scene) {
      for renderer in &self.renderer.renderer {
        if renderer
          .render_indirect_batch_models(
            indirect_batch.as_ref(),
            any_id,
            &camera,
            &self.renderer.texture_system,
            &self.pass,
            &mut cx.ctx,
          )
          .is_some()
        {
          return;
        }
      }
    }
  }
}
