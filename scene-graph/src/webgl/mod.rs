use crate::{RenderObject, Scene, SceneGraphBackEnd, SceneGraphRenderEngine};
use web_sys::*;

pub mod renderer;
pub use renderer::*;

pub struct SceneGraphWebGLBackend {
  engine: SceneGraphRenderEngine,
}

impl SceneGraphBackEnd for SceneGraphWebGLBackend {
  type RenderTarget = Option<WebGlFramebuffer>;
  type Renderer = WebGLRenderer;
  type Shading = WebGlProgram;
  type ShadingParameterGroup = ();
  type IndexBuffer = Option<WebGlBuffer>;
  type VertexBuffer = WebGLVertexBuffer;
  type UniformBuffer = WebGlBuffer;
}

pub struct WebGLVertexAttributeBuffer {
  buffer: WebGlBuffer,
  location: u32,
  desciptor: WebGLVertexAttributeBufferDescriptor,
}

pub struct WebGLVertexAttributeBufferDescriptor {
  offset: i32,
  size: i32,
  data_type: WebGLVertexAttributeDataType,
}

pub enum WebGLVertexAttributeDataType {
  Float,
}

impl WebGLVertexAttributeDataType {
  pub fn to_webgl(&self) -> u32 {
    match self {
      Self::Float => WebGl2RenderingContext::FLOAT,
    }
  }
}

pub struct WebGLVertexBuffer {
  stride: i32,
  attributes: Vec<WebGLVertexAttributeBuffer>, // todo use smallvec opt
                                               // todo optional VAO
}

impl SceneGraphWebGLBackend {
  pub fn new() -> Self {
    Self {
      engine: SceneGraphRenderEngine::new(),
    }
  }

  pub fn render(
    &mut self,
    scene: &mut Scene<SceneGraphWebGLBackend>,
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
  pub fn render_webgl(&self, renderer: &mut WebGLRenderer, scene: &Scene<SceneGraphWebGLBackend>) {
    let shading = scene.resources.get_shading(self.shading_index).resource();
    let geometry = &scene.resources.get_geometry(self.geometry_index).resource();

    renderer.use_program(&shading.gpu);

    // geometry bind
    geometry.index_buffer.map(|b| {
      let index = scene.resources.get_index_buffer(b);
      renderer.set_index_buffer(index.resource().as_ref());
    });
    for (i, vertex_buffer) in geometry.vertex_buffers.iter().enumerate() {
      let buffer = scene.resources.get_vertex_buffer(*vertex_buffer);
      renderer.set_vertex_buffer(i, buffer.resource());
    }

    // shading bind
    for i in 0..shading.get_parameters_count() {
      let _parameter_group = scene
        .resources
        .get_shading_param_group(shading.get_parameter(i));
      // pass.set_bindgroup(i, bindgroup.gpu());
      // todo!()
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
