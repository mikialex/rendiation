use crate::{RenderEngine, RenderObject, Scene, SceneGraphBackend, ShadingParameterType};
use web_sys::*;

pub mod attribute;
pub mod cal;
pub mod program;
pub mod renderer;
pub mod uniform;

pub use attribute::*;
pub use cal::*;
pub use program::*;
pub use renderer::*;
pub use uniform::*;

pub struct WebGLBackend {
  engine: RenderEngine<WebGLBackend>,
}

impl SceneGraphBackend for WebGLBackend {
  type RenderTarget = Option<WebGlFramebuffer>;
  type Renderer = WebGLRenderer;
  type Shading = WebGLProgram;
  type ShadingParameterGroup = ();
  type IndexBuffer = Option<WebGlBuffer>;
  type VertexBuffer = WebGLVertexBuffer;
  type UniformBuffer = WebGlBuffer;
  type UniformValue = UniformValue;
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

impl RenderObject<WebGLBackend> {
  pub fn render_webgl(&self, renderer: &mut WebGLRenderer, scene: &Scene<WebGLBackend>) {
    let resources = &scene.resources;
    let shading = resources.get_shading(self.shading_index).resource();
    let geometry = &resources.get_geometry(self.geometry_index).resource();
    let program = &shading.gpu;

    renderer.use_program(&shading.gpu.program());

    // geometry bind
    renderer.attribute_states.prepare_new_bindings();
    geometry.index_buffer.map(|b| {
      let index = resources.get_index_buffer(b);
      renderer.set_index_buffer(index.resource().as_ref());
    });
    geometry.vertex_buffers.iter().for_each(|v| {
      let buffer = resources.get_vertex_buffer(v.1).resource();
      let att_location = program.query_attribute_location(v.0);
      renderer.set_vertex_buffer(att_location, buffer);
    });
    renderer
      .attribute_states
      .disable_old_unused_bindings(&renderer.gl);

    // shading bind
    for i in 0..shading.get_parameters_count() {
      let parameter_group = resources
        .get_shading_param_group(shading.get_parameter(i))
        .resource();
      parameter_group.items.iter().for_each(|p| {
        use ShadingParameterType::*;
        match p.1 {
          UniformBuffer(index) => {
            let _uniform = resources.get_uniform(index).resource();
            todo!()
          }
          UniformValue(index) => {
            let uniform_value = resources.get_uniform_value(index).resource();
            // program.upload_uniform_value(uniform_value, renderer);
            todo!()
          }
          // SampledTexture(_index) => todo!(),
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
