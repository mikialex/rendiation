use crate::*;

pub struct GLESRenderSystem {
  pub model_lookup: UpdateResultToken,
  pub node_net_visible: UpdateResultToken,
  pub texture_system: TextureGPUSystemSource,
  pub camera: Box<dyn RenderImplProvider<Box<dyn CameraRenderImpl>>>,
  pub scene_model_impl: Vec<Box<dyn RenderImplProvider<Box<dyn SceneModelRenderer>>>>,
}

pub fn build_default_gles_render_system(prefer_bindless: bool) -> GLESRenderSystem {
  GLESRenderSystem {
    model_lookup: Default::default(),
    texture_system: TextureGPUSystemSource::new(prefer_bindless),
    node_net_visible: Default::default(),
    camera: Box::new(DefaultGLESCameraRenderImplProvider::default()),
    scene_model_impl: vec![Box::new(GLESPreferredComOrderRendererProvider {
      node: Box::new(DefaultGLESNodeRenderImplProvider::default()),
      model_impl: vec![Box::new(DefaultSceneStdModelRendererProvider {
        materials: vec![
          Box::new(PbrMRMaterialDefaultRenderImplProvider::default()),
          Box::new(PbrSGMaterialDefaultRenderImplProvider::default()),
          Box::new(FlatMaterialDefaultRenderImplProvider::default()),
        ],
        shapes: vec![Box::new(
          AttributesMeshEntityDefaultRenderImplProvider::default(),
        )],
      })],
    })],
  }
}

impl RenderImplProvider<Box<dyn SceneRenderer<ContentKey = SceneContentKey>>> for GLESRenderSystem {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.texture_system.register_resource(source, cx);
    let model_lookup = global_rev_ref().watch_inv_ref::<SceneModelBelongsToScene>();
    self.model_lookup = source.register_multi_reactive_query(model_lookup);
    self.camera.register_resource(source, cx);
    for imp in &mut self.scene_model_impl {
      imp.register_resource(source, cx);
    }
    self.node_net_visible = source.register_reactive_query(scene_node_derive_visible());
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    self.texture_system.deregister_resource(source);
    self.camera.deregister_resource(source);
    for imp in &mut self.scene_model_impl {
      imp.deregister_resource(source);
    }
    source.deregister(&mut self.model_lookup);
    source.deregister(&mut self.node_net_visible);
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn SceneRenderer<ContentKey = SceneContentKey>> {
    Box::new(GLESSceneRenderer {
      scene_model_renderer: self
        .scene_model_impl
        .iter()
        .map(|imp| imp.create_impl(res))
        .collect(),
      background: SceneBackgroundRenderer::new_from_global(),
      model_lookup: res
        .take_reactive_multi_query_updated(self.model_lookup)
        .unwrap(),
      texture_system: self.texture_system.create_impl(res),
      camera: self.camera.create_impl(res),
      node_net_visible: res
        .take_reactive_query_updated(self.node_net_visible)
        .unwrap(),
      sm_ref_node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
    })
  }
}

struct GLESSceneRenderer {
  texture_system: GPUTextureBindingSystem,
  camera: Box<dyn CameraRenderImpl>,
  scene_model_renderer: Vec<Box<dyn SceneModelRenderer>>,
  background: SceneBackgroundRenderer,
  model_lookup: RevRefOfForeignKey<SceneModelBelongsToScene>,
  node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  sm_ref_node: ForeignKeyReadView<SceneModelRefNode>,
}

#[derive(Clone)]
struct HostModelLookUp {
  v: RevRefOfForeignKey<SceneModelBelongsToScene>,
  node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  sm_ref_node: ForeignKeyReadView<SceneModelRefNode>,
  scene_id: EntityHandle<SceneEntity>,
}

impl HostRenderBatch for HostModelLookUp {
  fn iter_scene_models(&self) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + '_> {
    let iter = self.v.access_multi_value_dyn(&self.scene_id).filter(|sm| {
      let node = self.sm_ref_node.get(*sm).unwrap();
      self.node_net_visible.access(&node).unwrap_or(false)
    });
    Box::new(iter)
  }
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
    self
      .scene_model_renderer
      .render_scene_model(idx, camera, pass, cx, tex)
  }
}

impl SceneRenderer for GLESSceneRenderer {
  type ContentKey = SceneContentKey;

  fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    _semantic: Self::ContentKey, // todo
    _ctx: &mut FrameCtx,
  ) -> SceneModelRenderBatch {
    SceneModelRenderBatch::Host(Box::new(HostModelLookUp {
      v: self.model_lookup.clone(),
      node_net_visible: self.node_net_visible.clone(),
      sm_ref_node: self.sm_ref_node.clone(),
      scene_id: scene,
    }))
  }

  fn make_scene_batch_pass_content<'a>(
    &'a self,
    batch: SceneModelRenderBatch,
    camera: EntityHandle<SceneCameraEntity>,
    pass: &'a dyn RenderComponent,
    _ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a> {
    Box::new(GLESScenePassContent {
      renderer: self,
      batch: batch.get_host_batch().unwrap(),
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

struct GLESScenePassContent<'a> {
  renderer: &'a GLESSceneRenderer,
  batch: Box<dyn HostRenderBatch>,
  pass: &'a dyn RenderComponent,
  camera: EntityHandle<SceneCameraEntity>,
}

impl<'a> PassContent for GLESScenePassContent<'a> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let mut models = self.batch.iter_scene_models();

    let base = default_dispatcher(pass);
    let p = RenderArray([&base, self.pass] as [&dyn rendiation_webgpu::RenderComponent; 2]);

    self.renderer.render_reorderable_models(
      &mut models,
      self.camera,
      &p,
      &mut pass.ctx,
      &self.renderer.texture_system,
    );
  }
}
