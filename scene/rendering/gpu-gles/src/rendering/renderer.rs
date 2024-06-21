use rendiation_texture_core::TextureSampler;
use rendiation_texture_gpu_base::*;

use crate::*;

pub struct GLESRenderSystem {
  pub model_lookup: UpdateResultToken,
  pub texture_system: UpdateResultToken,
  pub camera: Box<dyn RenderImplProvider<Box<dyn GLESCameraRenderImpl>>>,
  pub scene_model_impl: Vec<Box<dyn RenderImplProvider<Box<dyn SceneModelRenderer>>>>,
}

pub fn build_default_gles_render_system() -> GLESRenderSystem {
  GLESRenderSystem {
    model_lookup: Default::default(),
    texture_system: Default::default(),
    camera: Box::new(DefaultGLESCameraRenderImplProvider::default()),
    scene_model_impl: vec![Box::new(GLESPreferredComOrderRendererProvider {
      node: Box::new(DefaultGLESNodeRenderImplProvider::default()),
      model_impl: vec![Box::new(DefaultSceneStdModelRendererProvider {
        materials: vec![
          Box::new(PbrMRMaterialDefaultRenderImplProvider::default()),
          Box::new(FlatMaterialDefaultRenderImplProvider::default()),
        ],
        shapes: vec![Box::new(AttributeMeshDefaultRenderImplProvider::default())],
      })],
    })],
  }
}

impl RenderImplProvider<Box<dyn SceneRenderer>> for GLESRenderSystem {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPUResourceCtx) {
    let default_2d: GPU2DTextureView = create_fallback_empty_texture(&cx.device)
      .create_default_view()
      .try_into()
      .unwrap();
    let texture_2d = gpu_texture_2ds(cx, default_2d.clone());

    let default_sampler = create_gpu_sampler(cx, &TextureSampler::default());
    let samplers = sampler_gpus(cx);

    let bindless_minimal_effective_count = 8192;
    self.texture_system =
      if is_bindless_supported_on_this_gpu(&cx.info, bindless_minimal_effective_count) {
        let texture_system = BindlessTextureSystemSource::new(
          texture_2d,
          default_2d,
          samplers,
          default_sampler,
          bindless_minimal_effective_count,
        );

        source.register(Box::new(ReactiveQueryBoxAnyResult(texture_system)))
      } else {
        let texture_system = TraditionalPerDrawBindingSystemSource {
          textures: Box::new(texture_2d),
          samplers: Box::new(samplers),
        };
        source.register(Box::new(ReactiveQueryBoxAnyResult(texture_system)))
      };

    let model_lookup = global_rev_ref().watch_inv_ref::<SceneModelBelongsToScene>();
    self.model_lookup = source.register_reactive_multi_collection(model_lookup);
    self.camera.register_resource(source, cx);
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
      texture_system: *res
        .take_result(self.texture_system)
        .unwrap()
        .downcast::<GPUTextureBindingSystem>()
        .unwrap(),
      camera: self.camera.create_impl(res),
    })
  }
}

struct GLESSceneRenderer {
  texture_system: GPUTextureBindingSystem,
  camera: Box<dyn GLESCameraRenderImpl>,
  scene_model_renderer: Vec<Box<dyn SceneModelRenderer>>,
  model_lookup:
    Box<dyn VirtualMultiCollection<EntityHandle<SceneEntity>, EntityHandle<SceneModelEntity>>>,
}

impl SceneModelRenderer for GLESSceneRenderer {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    camera_gpu: &'a (dyn GLESCameraRenderImpl + 'a),
    pass: &'a (dyn RenderComponent + 'a),
    tex: &'a GPUTextureBindingSystem,
  ) -> Option<(Box<dyn RenderComponent + 'a>, DrawCommand)> {
    self
      .scene_model_renderer
      .make_component(idx, camera, camera_gpu, pass, tex)
  }
}

impl SceneRenderer for GLESSceneRenderer {
  fn make_pass_content<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
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
    _scene: EntityHandle<SceneEntity>, // todo background
  ) -> (Operations<rendiation_webgpu::Color>, Operations<f32>) {
    (clear(rendiation_webgpu::Color::WHITE), clear(1.))
  }

  fn get_scene_model_cx(&self) -> &GPUTextureBindingSystem {
    &self.texture_system
  }

  fn setup_camera_jitter(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
    jitter: Vec2<f32>,
    queue: &GPUQueue,
  ) {
    self.camera.setup_camera_jitter(camera, jitter, queue)
  }

  fn render_reorderable_models(
    &self,
    models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
    camera: EntityHandle<SceneCameraEntity>,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) {
    self.render_reorderable_models_impl(models, camera, self.camera.as_ref(), pass, cx, tex)
  }
}

struct GLESScenePassContent<'a> {
  renderer: &'a GLESSceneRenderer,
  scene: EntityHandle<SceneEntity>,
  pass: &'a dyn RenderComponent,
  camera: EntityHandle<SceneCameraEntity>,
}

impl<'a> PassContent for GLESScenePassContent<'a> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let mut models = self.renderer.model_lookup.access_multi_value(&self.scene);

    self.renderer.render_reorderable_models(
      &mut models,
      self.camera,
      &self.pass,
      &mut pass.ctx,
      &self.renderer.texture_system,
    );
  }
}
