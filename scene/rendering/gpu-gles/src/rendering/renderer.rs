use crate::*;

pub struct GLESRenderSystem {
  pub scene_model_impl: Vec<Box<dyn RenderImplProvider<Box<dyn GLESSceneModelRenderImpl>>>>,
}

pub fn build_default_gles_render_system() -> GLESRenderSystem {
  GLESRenderSystem {
    scene_model_impl: vec![Box::new(GLESPreferredComOrderRendererProvider {
      node: Box::new(DefaultGLESNodeRenderImplProvider),
      camera: Box::new(DefaultGLESCameraRenderImplProvider),
      model_impl: vec![Box::new(DefaultSceneStdModelRendererProvider {
        materials: vec![
          Box::new(PbrMRMaterialDefaultRenderImplProvider),
          Box::new(FlatMaterialDefaultRenderImplProvider),
        ],
        shapes: vec![Box::new(AttributeMeshDefaultRenderImplProvider)],
      })],
    })],
  }
}

impl RenderImplProvider<Box<dyn SceneRenderer>> for GLESRenderSystem {
  fn register_resource(&self, res: &mut ReactiveResourceManager) {
    for imp in &self.scene_model_impl {
      imp.register_resource(res);
    }
  }

  fn create_impl(&self, res: &ResourceUpdateResult) -> Box<dyn SceneRenderer> {
    Box::new(GLESSceneRenderer {
      scene_model_renderer: self
        .scene_model_impl
        .iter()
        .map(|imp| imp.create_impl(res))
        .collect(),
      model_lookup: todo!(),
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
    pass: &dyn RenderComponentAny,
    ctx: &mut FrameCtx,
    target: RenderPassDescriptorOwned,
  ) {
    let mut ctx = ctx.encoder.begin_render_pass_with_info(target, ctx.gpu);
    for idx in self.model_lookup.access_multi_value(&scene) {
      let com = self.scene_model_renderer.make_component(idx, camera, pass);
      let command = self.scene_model_renderer.draw_command(idx);
      if let Some(com) = com {
        if let Some(command) = command {
          com.render(&mut ctx.ctx, command)
        }
      }
    }
  }
}
