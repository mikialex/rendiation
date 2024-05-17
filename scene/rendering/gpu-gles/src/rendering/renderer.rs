use rendiation_texture_gpu_system::GPUTextureBindingSystem;

use crate::*;

pub struct GLESRenderSystem {
  pub model_lookup: UpdateResultToken,
  pub scene_model_impl: Vec<Box<dyn RenderImplProvider<Box<dyn SceneModelRenderer>>>>,
}

pub fn build_default_gles_render_system(cx: &GPU) -> GLESRenderSystem {
  let texture_system = GPUTextureBindingSystem::new(cx, true, 8192);
  GLESRenderSystem {
    model_lookup: Default::default(),
    scene_model_impl: vec![Box::new(GLESPreferredComOrderRendererProvider {
      node: Box::new(DefaultGLESNodeRenderImplProvider::default()),
      camera: Box::new(DefaultGLESCameraRenderImplProvider::default()),
      model_impl: vec![Box::new(DefaultSceneStdModelRendererProvider {
        materials: vec![
          Box::new(PbrMRMaterialDefaultRenderImplProvider::new(texture_system)),
          Box::new(FlatMaterialDefaultRenderImplProvider::default()),
        ],
        shapes: vec![Box::new(AttributeMeshDefaultRenderImplProvider::default())],
      })],
    })],
  }
}

impl RenderImplProvider<Box<dyn SceneRenderer>> for GLESRenderSystem {
  fn register_resource(&mut self, source: &mut ReactiveStateJoinUpdater, cx: &GPUResourceCtx) {
    let model_lookup = global_rev_ref().watch_inv_ref_typed::<SceneModelBelongsToScene>();
    self.model_lookup = source.register_reactive_multi_collection(model_lookup);
    for imp in &mut self.scene_model_impl {
      imp.register_resource(source, cx);
    }
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> Box<dyn SceneRenderer> {
    Box::new(GLESSceneRenderer {
      scene_model_renderer: self
        .scene_model_impl
        .iter()
        .map(|imp| imp.create_impl(res))
        .collect(),
      model_lookup: res
        .take_multi_reactive_collection_updated(self.model_lookup)
        .unwrap(),
    })
  }
}

struct GLESSceneRenderer {
  scene_model_renderer: Vec<Box<dyn SceneModelRenderer>>,
  model_lookup: Box<dyn VirtualMultiCollection<AllocIdx<SceneEntity>, AllocIdx<SceneModelEntity>>>,
}

impl SceneModelRenderer for GLESSceneRenderer {
  fn make_component<'a>(
    &'a self,
    idx: AllocIdx<SceneModelEntity>,
    camera: AllocIdx<SceneCameraEntity>,
    pass: &'a (dyn RenderComponent + 'a),
  ) -> Option<(Box<dyn RenderComponent + 'a>, DrawCommand)> {
    self.scene_model_renderer.make_component(idx, camera, pass)
  }
}

impl SceneRenderer for GLESSceneRenderer {
  fn make_pass_content<'a>(
    &'a self,
    scene: AllocIdx<SceneEntity>,
    camera: AllocIdx<SceneCameraEntity>,
    pass: &'a dyn RenderComponent,
    _: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a> {
    Box::new(GLESScenePassContent {
      renderer: self,
      scene,
      pass,
      camera,
    })
  }
  fn init_clear(
    &self,
    _scene: AllocIdx<SceneEntity>, // todo background
  ) -> (Operations<rendiation_webgpu::Color>, Operations<f32>) {
    (clear(rendiation_webgpu::Color::WHITE), clear(1.))
  }
}

struct GLESScenePassContent<'a> {
  renderer: &'a GLESSceneRenderer,
  scene: AllocIdx<SceneEntity>,
  pass: &'a dyn RenderComponent,
  camera: AllocIdx<SceneCameraEntity>,
}

impl<'a> PassContent for GLESScenePassContent<'a> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let mut models = self.renderer.model_lookup.access_multi_value(&self.scene);

    self
      .renderer
      .render_reorderable_models(&mut models, self.camera, &self.pass, &mut pass.ctx)
  }
}
