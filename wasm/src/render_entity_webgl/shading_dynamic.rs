use rendiation_render_entity::*;
use crate::webgl::renderer::uploadMatrix4f;
use crate::webgl::*;
use fnv::FnvHasher;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::rc::Rc;
use web_sys::*;

impl Shading<WebGLRenderer> for DynamicShading {
  fn get_index(&self) -> usize {
    self.index
  }
  fn get_vertex_str(&self) -> &str {
    &self.vertex_str
  }
  fn get_fragment_str(&self) -> &str {
    &self.frag_str
  }

  fn make_gpu_port(&self, backend: &WebGLRenderer) -> Rc<dyn ShadingGPUPort<WebGLRenderer>> {
    Rc::new(
      DynamicProgram::new(
        backend.gl.clone(),
        self.get_vertex_str(),
        self.get_fragment_str(),
        &self.attributes,
        &self.uniforms,
        backend.step_id.get()
      )
      .unwrap(),
    )
  }
}

pub struct DynamicProgram {
  index: usize,
  context: Rc<WebGlRenderingContext>,
  pub program: WebGlProgram,
  pub uniforms: HashMap<String, WebGlUniformLocation, BuildHasherDefault<FnvHasher>>,
  pub attributes: HashMap<String, i32, BuildHasherDefault<FnvHasher>>,
}

impl ShadingGPUPort<WebGLRenderer> for DynamicProgram {
  fn get_index(&self) -> usize { self.index } 

  fn use_self(&self, renderer: &WebGLRenderer) {
    renderer.gl.use_program(Some(&self.program));
  }

  fn use_uniforms(&self, renderer: &WebGLRenderer) {
    let model_matrix_location = self.uniforms.get("model_matrix").unwrap();
    uploadMatrix4f(
      &renderer.gl,
      model_matrix_location,
      &renderer.model_transform,
    );

    let camera_inverse_location = self.uniforms.get("camera_inverse").unwrap();
    uploadMatrix4f(
      &renderer.gl,
      camera_inverse_location,
      &renderer.camera_inverse,
    );

    let projection_matrix_location = self.uniforms.get("projection_matrix").unwrap();
    uploadMatrix4f(
      &renderer.gl,
      projection_matrix_location,
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

impl DynamicProgram {
  pub fn new(
    context: Rc<WebGlRenderingContext>,
    vertex_shader_str: &str,
    frag_shader_str: &str,
    attributes_vec: &[String],
    uniforms_vec: &[String],
    index: usize,
  ) -> Result<DynamicProgram, String> {
    let program = make_webgl_program(&context, vertex_shader_str, frag_shader_str)?;

    // let activeUniform = context.get_program_parameter(&program, WebGlRenderingContext::ACTIVE_UNIFORMS);

    let mut uniforms = HashMap::with_hasher(BuildHasherDefault::<FnvHasher>::default());
    uniforms_vec.iter().for_each(|name| {
      uniforms.insert(
        name.clone(),
        context.get_uniform_location(&program, name).unwrap(),
      );
    });

    let mut attributes = HashMap::with_hasher(BuildHasherDefault::<FnvHasher>::default());
    attributes_vec.iter().for_each(|name| {
      attributes.insert(name.clone(), context.get_attrib_location(&program, name));
    });

    Ok(DynamicProgram {
      index,
      context,
      program,
      uniforms,
      attributes,
    })
  }
}
