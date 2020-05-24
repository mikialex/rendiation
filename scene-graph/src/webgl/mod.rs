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
  pub ctx: WebGlRenderingContext,
}

impl WebGLRenderer {
  pub fn use_program(&mut self, p: &WebGlProgram) {
    self.ctx.use_program(Some(p))
  }
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
    // todo!()
    let shading = scene.resources.get_shading(self.shading_index);
    let geometry = &scene.resources.get_geometry(self.geometry_index).data;

    renderer.use_program(shading.gpu());

    // geometry bind
    // pass.set_index_buffer(geometry.get_gpu_index_buffer());
    // for i in 0..geometry.vertex_buffer_count() {
    //   let buffer = geometry.get_gpu_vertex_buffer(i);
    //   pass.set_vertex_buffer(i, buffer);
    // }

    // shading bind
    for i in 0..shading.get_parameters_count() {
      let parameter_group = scene
        .resources
        .get_shading_param_group(shading.get_parameter(i));
      // pass.set_bindgroup(i, bindgroup.gpu());
    }

    let range = geometry.get_draw_range();
    renderer.ctx.draw_elements_with_i32(
      WebGlRenderingContext::TRIANGLES,
      range.start as i32,
      WebGlRenderingContext::UNSIGNED_INT,
      range.end as i32,
    );
  }
}
