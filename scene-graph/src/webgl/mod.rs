use crate::{RenderObject, Scene, SceneGraphBackEnd, SceneGraphRenderEngine};
use web_sys::*;

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
      Self::Float => WebGlRenderingContext::FLOAT,
    }
  }
}

pub struct WebGLVertexBuffer {
  stride: i32,
  attributes: Vec<WebGLVertexAttributeBuffer>, // todo use smallvec opt
                                               // todo optional VAO
}

pub struct WebGLRenderer {
  pub gl: WebGlRenderingContext,
}

impl WebGLRenderer {
  pub fn use_program(&mut self, p: &WebGlProgram) {
    self.gl.use_program(Some(p))
  }

  pub fn set_index_buffer(&self, buffer: Option<&WebGlBuffer>) {
    self
      .gl
      .bind_buffer(WebGlRenderingContext::ELEMENT_ARRAY_BUFFER, buffer)
  }

  pub fn set_vertex_buffer(&self, _index: usize, vertex_buffer: &WebGLVertexBuffer) {
    vertex_buffer.attributes.iter().for_each(|a| {
      self
        .gl
        .bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&a.buffer));
      self.gl.vertex_attrib_pointer_with_i32(
        a.location,
        a.desciptor.size,
        a.desciptor.data_type.to_webgl(),
        false,
        vertex_buffer.stride,
        a.desciptor.offset,
      );
      self.gl.enable_vertex_attrib_array(a.location);
    })
  }
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

// struct ShadingParameterGroup{

// }

impl RenderObject {
  pub fn render_webgl(&self, renderer: &mut WebGLRenderer, scene: &Scene<SceneGraphWebGLBackend>) {
    // todo!()
    let shading = scene.resources.get_shading(self.shading_index);
    let geometry = &scene.resources.get_geometry(self.geometry_index).data;

    renderer.use_program(shading.gpu());

    // geometry bind
    renderer.set_index_buffer(geometry.get_gpu_index_buffer().as_ref());
    for i in 0..geometry.vertex_buffer_count() {
      let buffer = geometry.get_gpu_vertex_buffer(i);
      renderer.set_vertex_buffer(i, buffer);
    }

    // shading bind
    for i in 0..shading.get_parameters_count() {
      let _parameter_group = scene
        .resources
        .get_shading_param_group(shading.get_parameter(i));
      // pass.set_bindgroup(i, bindgroup.gpu());
    }

    let range = geometry.get_draw_range();
    renderer.gl.draw_elements_with_i32(
      WebGlRenderingContext::TRIANGLES,
      range.start as i32,
      WebGlRenderingContext::UNSIGNED_INT,
      range.end as i32,
    );
  }
}
