use crate::*;

pub fn use_gles_scene_renderer(
  cx: &mut QueryGPUHookCx,
  reversed_depth: bool,
  attributes_custom_key: std::sync::Arc<dyn Fn(u32, &mut ShaderVertexBuilder)>,
  texture_system: Option<GPUTextureBindingSystem>,
) -> Option<GLESSceneRenderer> {
  let mesh = use_attribute_mesh_renderer(cx, attributes_custom_key).map(|v| Box::new(v) as Box<_>);

  let flat_mat = use_unlit_material_uniforms(cx);
  let pbr_mr_mat = use_pbr_mr_material_uniforms(cx);
  let pbr_sg_mat = use_pbr_sg_material_uniforms(cx);

  let std_model = std_model_renderer(cx, todo!(), mesh);

  let scene_model_renderer = use_gles_scene_model_renderer(cx, std_model);
  let model_lookup = cx.use_global_multi_reactive_query::<SceneModelBelongsToScene>();

  let node_net_visible = cx.use_reactive_query(scene_node_derive_visible);
  let model_alpha_blend = cx.use_reactive_query(all_kinds_of_materials_enabled_alpha_blending);

  cx.when_render(|| GLESSceneRenderer {
    model_lookup: model_lookup.unwrap(),
    node_net_visible: node_net_visible.unwrap(),
    texture_system: texture_system.unwrap(),
    reversed_depth,
    scene_model_renderer: scene_model_renderer.unwrap(),
    alpha_blend: model_alpha_blend.unwrap(),
    sm_ref_node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
  })
}

pub struct GLESSceneRenderer {
  texture_system: GPUTextureBindingSystem,
  scene_model_renderer: Box<dyn SceneModelRenderer>,
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
