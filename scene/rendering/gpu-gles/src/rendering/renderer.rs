use rendiation_texture_gpu_system::GPUTextureBindingSystem;

use crate::*;

pub struct GLESRenderSystem {
  pub model_lookup: UpdateResultToken,
  pub scene_model_impl: Vec<Box<dyn RenderImplProvider<Box<dyn GLESSceneModelRenderImpl>>>>,
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
  fn register_resource(&mut self, source: &mut ConcurrentStreamContainer, cx: &GPUResourceCtx) {
    let model_lookup = global_rev_ref().watch_inv_ref_typed::<SceneModelBelongsToScene>();
    self.model_lookup = source.register_reactive_multi_collection(model_lookup);
    for imp in &mut self.scene_model_impl {
      imp.register_resource(source, cx);
    }
  }

  fn create_impl(&self, res: &ConcurrentStreamUpdateResult) -> Box<dyn SceneRenderer> {
    Box::new(GLESSceneRenderer {
      scene_model_renderer: self
        .scene_model_impl
        .iter()
        .map(|imp| imp.create_impl(res))
        .collect(),
      model_lookup: res
        .get_multi_reactive_collection_updated(self.model_lookup)
        .unwrap(),
    })
  }
}

struct GLESSceneRenderer {
  scene_model_renderer: Vec<Box<dyn GLESSceneModelRenderImpl>>,
  model_lookup: Box<dyn VirtualMultiCollection<AllocIdx<SceneEntity>, AllocIdx<SceneModelEntity>>>,
}

impl SceneRenderer for GLESSceneRenderer {
  fn render(
    &self,
    scene: AllocIdx<SceneEntity>,
    camera: AllocIdx<SceneCameraEntity>,
    pass: &dyn RenderComponent,
    ctx: &mut FrameCtx,
    target: RenderPassDescriptorOwned,
  ) {
    let mut ctx = ctx.encoder.begin_render_pass_with_info(target, ctx.gpu);
    for idx in self.model_lookup.access_multi_value(&scene) {
      if let Some((com, command)) = self.scene_model_renderer.make_component(idx, camera, pass) {
        com.render(&mut ctx.ctx, command)
      }
    }
  }
}
