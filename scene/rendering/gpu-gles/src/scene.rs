use crate::*;

pub struct GLESSceneRenderer {
  pub texture_system: GPUTextureBindingSystem,
  pub scene_model_renderer: Box<dyn SceneModelRenderer>,
  pub reversed_depth: bool,
  pub model_error_state: SceneModelErrorRecorder,
}

impl SceneModelRenderer for GLESSceneRenderer {
  fn render_scene_model(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) -> Result<(), UnableToRenderSceneModelError> {
    let r = self
      .scene_model_renderer
      .render_scene_model(idx, camera, pass, cx, tex);
    self.model_error_state.report_and_filter_error(idx, &r);
    r
  }
}

impl SceneRenderer for GLESSceneRenderer {
  fn make_scene_batch_pass_content<'a>(
    &'a self,
    batch: SceneModelRenderBatch,
    camera: &'a dyn RenderComponent,
    pass: &'a dyn RenderComponent,
    _ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a> {
    Box::new(GLESScenePassContent {
      renderer: self,
      batch: batch.get_host_batch().unwrap(),
      pass,
      camera,
      reversed_depth: self.reversed_depth,
    })
  }
}

struct GLESScenePassContent<'a> {
  renderer: &'a GLESSceneRenderer,
  batch: Box<dyn HostRenderBatch>,
  pass: &'a dyn RenderComponent,
  camera: &'a dyn RenderComponent,
  reversed_depth: bool,
}

impl PassContent for GLESScenePassContent<'_> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let base = default_dispatcher(pass, self.reversed_depth).disable_auto_write();
    let p = RenderArray([&base, self.pass] as [&dyn rendiation_webgpu::RenderComponent; 2]);

    for sm in self.batch.iter_scene_models() {
      let _ = self.renderer.render_scene_model(
        sm,
        &self.camera,
        &p,
        &mut pass.ctx,
        &self.renderer.texture_system,
      );
    }
  }
}
