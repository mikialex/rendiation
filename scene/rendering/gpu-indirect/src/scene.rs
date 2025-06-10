use std::hash::Hasher;

use fast_hash_collection::FastHashMap;

use crate::*;

pub fn use_indirect_renderer(
  cx: &mut impl QueryGPUHookCx,
  reversed_depth: bool,
  materials: Option<Box<dyn IndirectModelMaterialRenderImpl>>,
  texture_system: Option<GPUTextureBindingSystem>,
) -> Option<IndirectSceneRenderer> {
  let mesh = use_bindless_mesh(cx).map(|v| Box::new(v) as Box<dyn IndirectModelShapeRenderImpl>);

  let std_model = use_std_model_renderer(cx, materials, mesh);

  let scene_model = use_indirect_scene_model(cx, std_model.map(|v| Box::new(v) as Box<_>));

  // todo, reuse with gles renderer
  let node_net_visible = cx.use_reactive_query(scene_node_derive_visible);
  let model_alpha_blend = cx.use_reactive_query(all_kinds_of_materials_enabled_alpha_blending);
  let model_lookup = cx.use_global_multi_reactive_query::<SceneModelBelongsToScene>();

  cx.when_render(|| IndirectSceneRenderer {
    texture_system: texture_system.unwrap(),
    renderer: scene_model.map(|v| Box::new(v) as Box<_>).unwrap(),
    node_net_visible: node_net_visible.unwrap(),
    sm_ref_node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
    model_lookup: model_lookup.unwrap(),
    reversed_depth,
    alpha_blend: model_alpha_blend.unwrap(),
  })
}

pub struct IndirectSceneRenderer {
  texture_system: GPUTextureBindingSystem,
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
    camera: &'a dyn RenderComponent,
    pass: &'a dyn RenderComponent,
    ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a> {
    let batch = self.create_batch_from_iter(models.iter_scene_models());
    self.make_scene_batch_pass_content(batch, camera, pass, ctx)
  }

  fn make_scene_batch_pass_content<'a>(
    &'a self,
    batch: SceneModelRenderBatch,
    camera: &'a dyn RenderComponent,
    pass: &'a dyn RenderComponent,
    ctx: &mut FrameCtx,
  ) -> Box<dyn PassContent + 'a> {
    let batch = batch.get_device_batch(None).unwrap();

    let content: Vec<_> = batch
      .sub_batches
      .iter()
      .map(|batch| {
        let provider = self.renderer.generate_indirect_draw_provider(batch, ctx);
        (provider, batch.impl_select_id)
      })
      .collect();

    Box::new(IndirectScenePassContent {
      renderer: self,
      content,
      pass,
      camera,
      reversed_depth: self.reversed_depth,
    })
  }
}

struct IndirectScenePassContent<'a> {
  renderer: &'a IndirectSceneRenderer,
  content: Vec<(
    Box<dyn IndirectDrawProvider>,
    EntityHandle<SceneModelEntity>,
  )>,

  pass: &'a dyn RenderComponent,
  camera: &'a dyn RenderComponent,
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
