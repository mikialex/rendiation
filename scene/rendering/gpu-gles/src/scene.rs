use crate::*;

pub fn use_gles_scene_renderer(
  cx: &mut impl QueryGPUHookCx,
  reversed_depth: bool,
  attributes_custom_key: std::sync::Arc<dyn Fn(u32, &mut ShaderVertexBuilder)>,
  texture_system: Option<GPUTextureBindingSystem>,
) -> Option<GLESSceneRenderer> {
  let mesh = use_attribute_mesh_renderer(cx, attributes_custom_key).map(|v| Box::new(v) as Box<_>);

  let unlit_mat = use_unlit_material_uniforms(cx);
  let pbr_mr_mat = use_pbr_mr_material_uniforms(cx);
  let pbr_sg_mat = use_pbr_sg_material_uniforms(cx);

  let materials = cx.when_render(|| {
    Box::new(vec![
      Box::new(unlit_mat.unwrap()) as Box<dyn GLESModelMaterialRenderImpl>,
      Box::new(pbr_mr_mat.unwrap()),
      Box::new(pbr_sg_mat.unwrap()),
    ]) as Box<dyn GLESModelMaterialRenderImpl>
  });

  let std_model = std_model_renderer(cx, materials, mesh);

  let scene_model_renderer = use_gles_scene_model_renderer(cx, std_model);
  cx.when_render(|| GLESSceneRenderer {
    texture_system: texture_system.unwrap(),
    reversed_depth,
    scene_model_renderer: scene_model_renderer.unwrap(),
  })
}

pub struct GLESSceneRenderer {
  texture_system: GPUTextureBindingSystem,
  scene_model_renderer: Box<dyn SceneModelRenderer>,
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
