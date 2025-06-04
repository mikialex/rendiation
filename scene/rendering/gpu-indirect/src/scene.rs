use std::hash::Hasher;

use fast_hash_collection::FastHashMap;

use crate::*;

pub struct IndirectRenderSystem {
  pub model_lookup: QueryToken,
  pub model_alpha_blend: QueryToken,
  pub node_net_visible: QueryToken,
  pub texture_system: TextureGPUSystemSource,
  pub scene_model_impl: IndirectPreferredComOrderRendererProvider,
  pub reversed_depth: bool,
}

impl IndirectRenderSystem {
  pub fn new(
    texture_impl_ty: GPUTextureBindingSystemType,
    reversed_depth: bool,
    scene_model_impl: IndirectPreferredComOrderRendererProvider,
  ) -> Self {
    IndirectRenderSystem {
      reversed_depth,
      model_lookup: Default::default(),
      node_net_visible: Default::default(),
      model_alpha_blend: Default::default(),
      texture_system: TextureGPUSystemSource::new(texture_impl_ty),
      scene_model_impl,
    }
  }
}

// pub fn build_default_indirect_render_system(
//   gpu: &GPU,
//   prefer_bindless: bool,
//   camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
//   reversed_depth: bool,
// ) -> IndirectRenderSystem {
//   let tex_sys_ty = get_suitable_texture_system_ty(gpu, true, prefer_bindless);
//   IndirectRenderSystem::new(
//     tex_sys_ty,
//     reversed_depth,
//     camera_source,
//     IndirectPreferredComOrderRendererProvider::default().register_std_model_impl(
//       DefaultSceneStdModelIndirectRendererProvider::default()
//         .register_material_impl(UnlitMaterialDefaultIndirectRenderImplProvider::default())
//         .register_material_impl(PbrMRMaterialDefaultIndirectRenderImplProvider::default())
//         .register_material_impl(PbrSGMaterialDefaultIndirectRenderImplProvider::default())
//         .register_shape_impl(MeshBindlessGPUSystemSource::new(gpu)),
//     ),
//   )
// }

// impl QueryBasedFeature<Box<dyn SceneRenderer<ContentKey = SceneContentKey>>>
//   for IndirectRenderSystem
// {
//   type Context = GPU;
//   fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
//     self.texture_system.register_resource(qcx, cx);
//     self.background.register(qcx, cx);

//     let model_lookup = global_rev_ref().watch_inv_ref::<SceneModelBelongsToScene>();
//     self.model_lookup = qcx.register_multi_reactive_query(model_lookup);
//     self.camera.register(qcx, cx);
//     self.scene_model_impl.register(qcx, cx);
//     self.node_net_visible = qcx.register_reactive_query(scene_node_derive_visible());
//     self.model_alpha_blend =
//       qcx.register_reactive_query(all_kinds_of_materials_enabled_alpha_blending());
//   }

//   fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
//     self.texture_system.deregister_resource(qcx);
//     self.background.deregister(qcx);
//     self.camera.deregister(qcx);
//     self.scene_model_impl.deregister(qcx);
//     qcx.deregister(&mut self.model_lookup);
//     qcx.deregister(&mut self.node_net_visible);
//     qcx.deregister(&mut self.model_alpha_blend);
//   }

//   fn create_impl(
//     &self,
//     cx: &mut QueryResultCtx,
//   ) -> Box<dyn SceneRenderer<ContentKey = SceneContentKey>> {
//     Box::new(IndirectSceneRenderer {
//       texture_system: self.texture_system.create_impl(cx),
//       renderer: self.scene_model_impl.create_impl(cx),
//       node_net_visible: cx
//         .take_reactive_query_updated(self.node_net_visible)
//         .unwrap(),
//       sm_ref_node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
//       model_lookup: cx
//         .take_reactive_multi_query_updated(self.model_lookup)
//         .unwrap(),
//       reversed_depth: self.reversed_depth,
//       alpha_blend: cx
//         .take_reactive_query_updated(self.model_alpha_blend)
//         .unwrap(),
//     })
//   }
// }

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
