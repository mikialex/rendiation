use crate::*;

pub struct GLESRenderSystem {
  pub model_lookup: QueryToken,
  pub node_net_visible: QueryToken,
  pub model_alpha_blend: QueryToken,
  pub texture_system: TextureGPUSystemSource,
  pub background: SceneBackgroundRendererSource,
  pub camera: BoxedQueryBasedGPUFeature<Box<dyn CameraRenderImpl>>,
  pub scene_model_impl: Vec<BoxedQueryBasedGPUFeature<Box<dyn SceneModelRenderer>>>,
  pub reversed_depth: bool,
}

pub fn build_default_gles_render_system(
  cx: &GPU,
  prefer_bindless: bool,
  camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
  reversed_depth: bool,
) -> GLESRenderSystem {
  let tex_sys_ty = get_suitable_texture_system_ty(cx, false, prefer_bindless);
  GLESRenderSystem {
    reversed_depth,
    model_lookup: Default::default(),
    model_alpha_blend: Default::default(),
    texture_system: TextureGPUSystemSource::new(tex_sys_ty),
    background: Default::default(),
    node_net_visible: Default::default(),
    camera: Box::new(DefaultGLESCameraRenderImplProvider::new(camera_source)),
    scene_model_impl: vec![Box::new(GLESPreferredComOrderRendererProvider {
      scene_model_ids: Default::default(),
      node: Box::new(DefaultGLESNodeRenderImplProvider::default()),
      model_impl: vec![Box::new(DefaultSceneStdModelRendererProvider {
        materials: vec![
          Box::new(PbrMRMaterialDefaultRenderImplProvider::default()),
          Box::new(PbrSGMaterialDefaultRenderImplProvider::default()),
          Box::new(UnlitMaterialDefaultRenderImplProvider::default()),
        ],
        shapes: vec![Box::new(
          AttributesMeshEntityDefaultRenderImplProvider::default(),
        )],
      })],
    })],
  }
}

impl QueryBasedFeature<Box<dyn SceneRenderer<ContentKey = SceneContentKey>>> for GLESRenderSystem {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    self.texture_system.register_resource(qcx, cx);
    self.background.register(qcx, cx);
    let model_lookup = global_rev_ref().watch_inv_ref::<SceneModelBelongsToScene>();
    self.model_lookup = qcx.register_multi_reactive_query(model_lookup);
    self.camera.register(qcx, cx);
    for imp in &mut self.scene_model_impl {
      imp.register(qcx, cx);
    }
    self.node_net_visible = qcx.register_reactive_query(scene_node_derive_visible());
    self.model_alpha_blend =
      qcx.register_reactive_query(all_kinds_of_materials_enabled_alpha_blending());
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    self.texture_system.deregister_resource(qcx);
    self.background.deregister(qcx);
    self.camera.deregister(qcx);
    for imp in &mut self.scene_model_impl {
      imp.deregister(qcx);
    }
    qcx.deregister(&mut self.model_lookup);
    qcx.deregister(&mut self.node_net_visible);
    qcx.deregister(&mut self.model_alpha_blend);
  }

  fn create_impl(
    &self,
    cx: &mut QueryResultCtx,
  ) -> Box<dyn SceneRenderer<ContentKey = SceneContentKey>> {
    Box::new(GLESSceneRenderer {
      scene_model_renderer: self
        .scene_model_impl
        .iter()
        .map(|imp| imp.create_impl(cx))
        .collect(),
      background: self.background.create_impl(cx),
      model_lookup: cx
        .take_reactive_multi_query_updated(self.model_lookup)
        .unwrap(),
      texture_system: self.texture_system.create_impl(cx),
      camera: self.camera.create_impl(cx),
      node_net_visible: cx
        .take_reactive_query_updated(self.node_net_visible)
        .unwrap(),
      sm_ref_node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
      reversed_depth: self.reversed_depth,
      alpha_blend: cx
        .take_reactive_query_updated(self.model_alpha_blend)
        .unwrap(),
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
  alpha_blend: BoxedDynQuery<EntityHandle<SceneModelEntity>, bool>,
  sm_ref_node: ForeignKeyReadView<SceneModelRefNode>,
  reversed_depth: bool,
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
    semantic: Self::ContentKey,
    _ctx: &mut FrameCtx,
  ) -> SceneModelRenderBatch {
    SceneModelRenderBatch::Host(Box::new(HostModelLookUp {
      v: self.model_lookup.clone(),
      node_net_visible: self.node_net_visible.clone(),
      sm_ref_node: self.sm_ref_node.clone(),
      scene_id: scene,
      scene_model_use_alpha_blending: self.alpha_blend.clone(),
      enable_alpha_blending: semantic.only_alpha_blend_objects,
    }))
  }

  fn make_scene_batch_pass_content<'a>(
    &'a self,
    batch: SceneModelRenderBatch,
    camera: CameraRenderSource,
    pass: &'a dyn RenderComponent,
    _ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a> {
    let camera = match camera {
      CameraRenderSource::Scene(camera) => self.get_camera_gpu().make_component(camera).unwrap(),
      CameraRenderSource::External(camera) => camera,
    };
    Box::new(GLESScenePassContent {
      renderer: self,
      batch: batch.get_host_batch().unwrap(),
      pass,
      camera,
      reversed_depth: self.reversed_depth,
    })
  }

  fn init_clear(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> (Operations<rendiation_webgpu::Color>, Operations<f32>) {
    self.background.init_clear(scene, self.reversed_depth)
  }
  fn render_background<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    camera: CameraRenderSource,
    tonemap: &'a dyn RenderComponent,
  ) -> Box<dyn PassContent + 'a> {
    let camera = match camera {
      CameraRenderSource::Scene(camera) => self.get_camera_gpu().make_component(camera).unwrap(),
      CameraRenderSource::External(camera) => camera,
    };
    Box::new(self.background.draw(scene, camera, tonemap))
  }

  fn get_camera_gpu(&self) -> &dyn CameraRenderImpl {
    self.camera.as_ref()
  }
}

struct GLESScenePassContent<'a> {
  renderer: &'a GLESSceneRenderer,
  batch: Box<dyn HostRenderBatch>,
  pass: &'a dyn RenderComponent,
  camera: Box<dyn RenderComponent + 'a>,
  reversed_depth: bool,
}

impl PassContent for GLESScenePassContent<'_> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let base = default_dispatcher(pass, self.reversed_depth).disable_auto_write();
    let p = RenderArray([&base, self.pass] as [&dyn rendiation_webgpu::RenderComponent; 2]);

    for sm in self.batch.iter_scene_models() {
      let r = self.renderer.render_scene_model(
        sm,
        &self.camera,
        &p,
        &mut pass.ctx,
        &self.renderer.texture_system,
      );
      if let Err(e) = r {
        println!("Failed to render scene model: {}", e);
      }
    }
  }
}
