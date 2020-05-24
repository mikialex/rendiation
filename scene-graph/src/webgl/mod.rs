use crate::{RenderObject, Scene, SceneGraphBackEnd, SceneGraphRenderEngine};
use web_sys::*;

pub struct SceneGraphWebGLRendererBackend {
  engine: SceneGraphRenderEngine,
}

impl SceneGraphBackEnd for SceneGraphWebGLRendererBackend {
  type RenderTarget = Option<WebGlFramebuffer>;
  type Renderer = WebGLRenderer;
  type Shading = WebGlProgram;
  type ShadingParameterGroup = ();
  type IndexBuffer = WebGlBuffer;
  type VertexBuffer = WebGlBuffer;
}

pub struct WebGLRenderer {
  ctx: WebGlRenderingContext,
}

impl SceneGraphWebGLRendererBackend {
  pub fn new() -> Self {
    Self {
      engine: SceneGraphRenderEngine::new(),
    }
  }

  pub fn render(
    &mut self,
    scene: &mut Scene<SceneGraphWebGLRendererBackend>,
    renderer: &mut WebGLRenderer,
    target: Option<WebGlFramebuffer>,
  ) {
    self.engine.update_render_list(scene);

    scene
      .background
      .as_ref()
      .map(|b| b.render(renderer, target));

    for drawcall in &self.engine.scene_raw_list.drawcalls {
      // let node = self.nodes.get(drawcall.node).unwrap();
      let render_obj = scene.render_objects.get(drawcall.render_object).unwrap();
      render_obj.render_webgl(renderer, scene);
    }
  }
}

impl RenderObject {
  pub fn render_webgl(
    &self,
    renderer: &mut WebGLRenderer,
    scene: &Scene<SceneGraphWebGLRendererBackend>,
  ) {
    todo!()
  }
}
