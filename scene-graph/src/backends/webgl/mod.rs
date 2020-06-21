use crate::{RenderObject, Scene, SceneGraphBackend, RenderEngine, ShadingParameterType};
use web_sys::*;

pub mod renderer;
pub mod attribute;
pub mod program;
pub mod uniform;
pub mod cal;

pub use renderer::*;
pub use attribute::*;
pub use program::*;
pub use uniform::*;
pub use cal::*;

pub struct WebGLBackend {
  engine: RenderEngine,
}

impl SceneGraphBackend for WebGLBackend {
  type RenderTarget = Option<WebGlFramebuffer>;
  type Renderer = WebGLRenderer;
  type Shading = WebGlProgram;
  type ShadingParameterGroup = ();
  type IndexBuffer = Option<WebGlBuffer>;
  type VertexBuffer = WebGLVertexBuffer;
  type UniformBuffer = WebGlBuffer;
}

impl WebGLBackend {
  pub fn new() -> Self {
    Self {
      engine: RenderEngine::new(),
    }
  }

  pub fn render(
    &mut self,
    scene: &mut Scene<WebGLBackend>,
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
  pub fn render_webgl(&self, renderer: &mut WebGLRenderer, scene: &Scene<WebGLBackend>) {
    let resources = &scene.resources;
    let shading = resources.get_shading(self.shading_index).resource();
    let geometry = &resources.get_geometry(self.geometry_index).resource();

    renderer.use_program(&shading.gpu);

    // geometry bind
    geometry.index_buffer.map(|b| {
      let index = resources.get_index_buffer(b);
      renderer.set_index_buffer(index.resource().as_ref());
    });
    for (i, vertex_buffer) in geometry.vertex_buffers.iter().enumerate() {
      let buffer = resources.get_vertex_buffer(*vertex_buffer);
      // we should make sure that the i is match the attribute location
      renderer.set_vertex_buffer(i, buffer.resource());
    }

    // shading bind
    for i in 0..shading.get_parameters_count() {
      let parameter_group = resources
        .get_shading_param_group(shading.get_parameter(i))
        .resource();
      parameter_group.items.iter().for_each(|p| {
        use ShadingParameterType::*;
        match p {
          UniformBuffer(index) => {
            let _uniform = resources.get_uniform(*index).resource();
            todo!()
          }
          SampledTexture(_index) => todo!(),
          _ => panic!("unsupported webgl resource type"),
        }
      })
    }

    let range = &geometry.draw_range;
    renderer.gl.draw_elements_with_i32(
      WebGl2RenderingContext::TRIANGLES,
      range.start as i32,
      WebGl2RenderingContext::UNSIGNED_INT,
      range.end as i32,
    );
  }
}
