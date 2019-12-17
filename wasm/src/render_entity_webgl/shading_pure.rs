use rendiation_render_entity::*;
use crate::webgl::programs::make_webgl_program;
use crate::webgl::renderer::uploadMatrix4f;
use crate::webgl::renderer::WebGLRenderer;
use core::hash::BuildHasherDefault;
use fnv::FnvHasher;
use std::collections::HashMap;
use std::rc::Rc;
use web_sys::WebGlProgram;
use web_sys::WebGlRenderingContext;
use web_sys::WebGlUniformLocation;


impl Shading<WebGLRenderer> for PureColorShading {
  fn get_index(&self) -> usize {
    self.index
  }
  fn get_vertex_str(&self) -> &str {
    &self.vertex
  }
  fn get_fragment_str(&self) -> &str {
    &self.frag
  }

  fn make_gpu_port(&self, backend: &WebGLRenderer) -> Rc<dyn ShadingGPUPort<WebGLRenderer>> {
    let gl = backend.gl.clone();
    let program = make_webgl_program(&gl, &self.vertex, &self.frag).unwrap();

    let mut attributes = HashMap::with_hasher(BuildHasherDefault::<FnvHasher>::default());
    vec![String::from("position")].iter().for_each(|name| {
      attributes.insert(name.clone(), gl.get_attrib_location(&program, name));
    });

    let projection_matrix = gl
      .get_uniform_location(&program, "projection_matrix")
      .unwrap();
    let world_matrix = gl.get_uniform_location(&program, "model_matrix").unwrap();
    let camera_inverse_matrix = gl.get_uniform_location(&program, "camera_inverse").unwrap();

    let p = PureColorProgram {
      index: backend.step_id.get(),
      program,
      projection_matrix,
      world_matrix,
      camera_inverse_matrix,
      attributes,
    };
    Rc::new(p)
  }
}

pub struct PureColorProgram {
  index: usize,
  program: WebGlProgram,
  projection_matrix: WebGlUniformLocation,
  world_matrix: WebGlUniformLocation,
  camera_inverse_matrix: WebGlUniformLocation,
  pub attributes: HashMap<String, i32, BuildHasherDefault<FnvHasher>>,
}

impl ShadingGPUPort<WebGLRenderer> for PureColorProgram {
  fn get_index(&self) -> usize { self.index } 

  fn use_self(&self, renderer: &WebGLRenderer) {
    renderer.gl.use_program(Some(&self.program));
  }

  fn use_uniforms(&self, renderer: &WebGLRenderer){
    uploadMatrix4f(&renderer.gl, &self.world_matrix, &renderer.model_transform);
    uploadMatrix4f(
      &renderer.gl,
      &self.camera_inverse_matrix,
      &renderer.camera_inverse,
    );
    uploadMatrix4f(
      &renderer.gl,
      &self.projection_matrix,
      &renderer.camera_projection,
    );
  }

  
  fn use_geometry(&self, renderer: &mut WebGLRenderer, geometry: Rc<dyn Geometry>) {
    let gl = renderer.gl.clone();
      if let Some(index) = &geometry.get_index_attribute() {
        let buffer = renderer
          .buffer_manager
          .get_index_buffer((*index).clone())
          .unwrap();
        
        gl.bind_buffer(WebGlRenderingContext::ELEMENT_ARRAY_BUFFER, Some(buffer));
      } else {
        gl.bind_buffer(WebGlRenderingContext::ELEMENT_ARRAY_BUFFER, None);
      }

      for (name, location) in self.attributes.iter() {
        let buffer_data = geometry.get_attribute_by_name(name).unwrap();
        let buffer = renderer.buffer_manager.get_buffer(buffer_data.clone()).unwrap();
        gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(buffer));
        gl.vertex_attrib_pointer_with_i32(
          *location as u32,
          buffer_data.stride as i32,
          WebGlRenderingContext::FLOAT,
          false,
          0,
          0,
        );
        gl.enable_vertex_attrib_array(*location as u32);
      }
  }

}
