use std::hash::Hasher;

use fast_hash_collection::FastHashMap;

use crate::*;

pub struct IndirectRenderSystem {
  pub model_lookup: UpdateResultToken,
  pub model_alpha_blend: UpdateResultToken,
  pub node_net_visible: UpdateResultToken,
  pub texture_system: TextureGPUSystemSource,
  pub background: SceneBackgroundRendererSource,
  pub camera: Box<dyn RenderImplProvider<Box<dyn CameraRenderImpl>>>,
  pub scene_model_impl: Box<dyn RenderImplProvider<Box<dyn IndirectBatchSceneModelRenderer>>>,
  pub reversed_depth: bool,
}

pub fn build_default_indirect_render_system(
  gpu: &GPU,
  prefer_bindless: bool,
  camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
  reversed_depth: bool,
) -> IndirectRenderSystem {
  let tex_sys_ty = get_suitable_texture_system_ty(gpu, true, prefer_bindless);
  IndirectRenderSystem {
    reversed_depth,
    model_lookup: Default::default(),
    node_net_visible: Default::default(),
    model_alpha_blend: Default::default(),
    background: SceneBackgroundRendererSource::new(reversed_depth),
    texture_system: TextureGPUSystemSource::new(tex_sys_ty),
    camera: Box::new(DefaultGLESCameraRenderImplProvider::new(camera_source)),
    scene_model_impl: Box::new(IndirectPreferredComOrderRendererProvider {
      ids: Default::default(),
      node: Box::new(DefaultIndirectNodeRenderImplProvider::default()),
      model_impl: vec![Box::new(DefaultSceneStdModelRendererProvider {
        std_model: Default::default(),
        materials: vec![
          Box::new(UnlitMaterialDefaultIndirectRenderImplProvider::default()),
          Box::new(PbrMRMaterialDefaultIndirectRenderImplProvider::default()),
          Box::new(PbrSGMaterialDefaultIndirectRenderImplProvider::default()),
        ],
        shapes: vec![Box::new(MeshBindlessGPUSystemSource::new(gpu))],
      })],
    }),
  }
}

impl RenderImplProvider<Box<dyn SceneRenderer<ContentKey = SceneContentKey>>>
  for IndirectRenderSystem
{
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.texture_system.register_resource(source, cx);
    self.background.register_resource(source, cx);

    let model_lookup = global_rev_ref().watch_inv_ref::<SceneModelBelongsToScene>();
    self.model_lookup = source.register_multi_reactive_query(model_lookup);
    self.camera.register_resource(source, cx);
    self.scene_model_impl.register_resource(source, cx);
    self.node_net_visible = source.register_reactive_query(scene_node_derive_visible());
    self.model_alpha_blend =
      source.register_reactive_query(all_kinds_of_materials_enabled_alpha_blending());
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    self.texture_system.deregister_resource(source);
    self.background.deregister_resource(source);
    self.camera.deregister_resource(source);
    self.scene_model_impl.deregister_resource(source);
    source.deregister(&mut self.model_lookup);
    source.deregister(&mut self.node_net_visible);
    source.deregister(&mut self.model_alpha_blend);
  }

  fn create_impl(
    &self,
    res: &mut QueryResultCtx,
  ) -> Box<dyn SceneRenderer<ContentKey = SceneContentKey>> {
    Box::new(IndirectSceneRenderer {
      texture_system: self.texture_system.create_impl(res),
      camera: self.camera.create_impl(res),
      background: self.background.create_impl(res),
      renderer: self.scene_model_impl.create_impl(res),
      node_net_visible: res
        .take_reactive_query_updated(self.node_net_visible)
        .unwrap(),
      sm_ref_node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
      model_lookup: res
        .take_reactive_multi_query_updated(self.model_lookup)
        .unwrap(),
      reversed_depth: self.reversed_depth,
      alpha_blend: res
        .take_reactive_query_updated(self.model_alpha_blend)
        .unwrap(),
    })
  }
}

struct IndirectSceneRenderer {
  texture_system: GPUTextureBindingSystem,
  camera: Box<dyn CameraRenderImpl>,
  background: SceneBackgroundRenderer,
  renderer: Box<dyn IndirectBatchSceneModelRenderer>,
  model_lookup: RevRefOfForeignKey<SceneModelBelongsToScene>,
  node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  alpha_blend: BoxedDynQuery<EntityHandle<SceneModelEntity>, bool>,
  sm_ref_node: ForeignKeyReadView<SceneModelRefNode>,
  reversed_depth: bool,
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

impl IndirectSceneRenderer {
  fn create_batch_from_iter(
    &self,
    iter: impl Iterator<Item = EntityHandle<SceneModelEntity>>,
  ) -> SceneModelRenderBatch {
    let mut classifier = FastHashMap::default();

    for sm in iter {
      let mut hasher = PipelineHasher::default();
      self
        .renderer
        .hash_shader_group_key_with_self_type_info(sm, &mut hasher)
        .expect("unable to find indirect group key for scene_model");
      let shader_hash = hasher.finish();
      let list = classifier.entry(shader_hash).or_insert_with(Vec::new);
      list.push(sm);
    }

    let sub_batches = classifier
      .drain()
      .map(|(_, list)| {
        let scene_models: Vec<_> = list.iter().map(|sm| sm.alloc_index()).collect();
        let scene_models = Box::new(scene_models);

        DeviceSceneModelRenderSubBatch {
          scene_models,
          impl_select_id: *list.first().unwrap(),
        }
      })
      .collect();

    SceneModelRenderBatch::Device(DeviceSceneModelRenderBatch {
      sub_batches,
      stash_culler: None,
    })
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
    let iter = HostModelLookUp {
      v: self.model_lookup.clone(),
      node_net_visible: self.node_net_visible.clone(),
      sm_ref_node: self.sm_ref_node.clone(),
      scene_id: scene,
      scene_model_use_alpha_blending: self.alpha_blend.clone(),
      enable_alpha_blending: semantic.only_alpha_blend_objects,
    };

    self.create_batch_from_iter(iter.iter_scene_models())
  }

  fn render_models<'a>(
    &'a self,
    models: Box<dyn HostRenderBatch>,
    camera: CameraRenderSource,
    pass: &'a dyn RenderComponent,
    ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a> {
    let batch = self.create_batch_from_iter(models.iter_scene_models());
    self.make_scene_batch_pass_content(batch, camera, pass, ctx)
  }

  fn make_scene_batch_pass_content<'a>(
    &'a self,
    batch: SceneModelRenderBatch,
    camera: CameraRenderSource,
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

    let camera = match camera {
      CameraRenderSource::Scene(camera) => self.camera.make_component(camera).unwrap(),
      CameraRenderSource::External(camera) => camera,
    };

    Box::new(IndirectScenePassContent {
      renderer: self,
      content,
      pass,
      camera,
      reversed_depth: self.reversed_depth,
    })
  }

  fn init_clear(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> (Operations<rendiation_webgpu::Color>, Operations<f32>) {
    self.background.init_clear(scene)
  }
  fn render_background(
    &self,
    scene: EntityHandle<SceneEntity>,
    camera: CameraRenderSource,
  ) -> Box<dyn PassContent + '_> {
    let camera = match camera {
      CameraRenderSource::Scene(camera) => self.get_camera_gpu().make_component(camera).unwrap(),
      CameraRenderSource::External(camera) => camera,
    };
    Box::new(self.background.draw(scene, camera))
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
  camera: Box<dyn RenderComponent + 'a>,
  reversed_depth: bool,
}

impl PassContent for IndirectScenePassContent<'_> {
  fn render(&mut self, cx: &mut FrameRenderPass) {
    let base = default_dispatcher(cx, self.reversed_depth).disable_auto_write();
    let p = RenderArray([&base, self.pass] as [&dyn rendiation_webgpu::RenderComponent; 2]);

    for (content, any_scene_model) in &self.content {
      self.renderer.renderer.render_indirect_batch_models(
        content.as_ref(),
        *any_scene_model,
        &self.camera,
        &self.renderer.texture_system,
        &p,
        &mut cx.ctx,
      );
    }
  }
}
