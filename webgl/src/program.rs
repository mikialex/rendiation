use crate::WebGLRenderer;
use rendiation_ral::*;
use std::collections::HashMap;
use web_sys::*;

pub struct WebGLProgram {
  program: WebGlProgram,
  attributes: HashMap<AttributeTypeId, i32>,
  uniforms: HashMap<UniformTypeId, WebGlUniformLocation>,
}

impl WebGLProgram {
  pub fn new(renderer: &mut WebGLRenderer, des: &SceneShadingDescriptor) -> Self {
    let gl = &renderer.gl;
    let program = make_webgl_program(
      &renderer.gl,
      &des.shader_descriptor.vertex_shader_str,
      &des.shader_descriptor.frag_shader_str,
    )
    .unwrap();

    let uniforms: HashMap<UniformTypeId, WebGlUniformLocation> = des
      .shader_descriptor
      .input_group()
      .iter()
      .flat_map(|d| d.inputs().iter())
      .map(|d| (d.id(), gl.get_uniform_location(&program, d.name()).unwrap()))
      .collect();

    let attributes: HashMap<AttributeTypeId, i32> = des
      .shader_descriptor
      .attribute_inputs()
      .iter()
      .flat_map(|d| d.attributes().iter())
      .map(|d| (d.id(), gl.get_attrib_location(&program, d.name())))
      .collect();

    WebGLProgram {
      program,
      attributes,
      uniforms,
    }
  }

  pub fn program(&self) -> &WebGlProgram {
    &self.program
  }

  pub fn query_uniform_location(&self, input_id: UniformTypeId) -> &WebGlUniformLocation {
    self.uniforms.get(&input_id).unwrap()
  }

  pub fn query_attribute_location(&self, input_id: AttributeTypeId) -> i32 {
    *self.attributes.get(&input_id).unwrap()
  }
}

impl WebGLRenderer {
  pub fn use_program(&mut self, p: &WebGlProgram) {
    self.gl.use_program(Some(p))
  }
}

pub fn make_webgl_program(
  context: &WebGl2RenderingContext,
  vertex_shader_str: &str,
  frag_shader_str: &str,
) -> Result<WebGlProgram, String> {
  let vertex_shader = compile_shader(
    &context,
    WebGl2RenderingContext::VERTEX_SHADER,
    vertex_shader_str,
  )?;
  let frag_shader = compile_shader(
    &context,
    WebGl2RenderingContext::FRAGMENT_SHADER,
    frag_shader_str,
  )?;
  link_program(&context, &vertex_shader, &frag_shader)
}

fn compile_shader(
  context: &WebGl2RenderingContext,
  shader_type: u32,
  source: &str,
) -> Result<WebGlShader, String> {
  let shader = context
    .create_shader(shader_type)
    .ok_or_else(|| String::from("Unable to create shader object"))?;
  context.shader_source(&shader, source);
  context.compile_shader(&shader);

  if context
    .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
    .as_bool()
    .unwrap_or(false)
  {
    Ok(shader)
  } else {
    Err(
      context
        .get_shader_info_log(&shader)
        .unwrap_or_else(|| String::from("Unknown error creating shader")),
    )
  }
}

fn link_program(
  context: &WebGl2RenderingContext,
  vert_shader: &WebGlShader,
  frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
  let program = context
    .create_program()
    .ok_or_else(|| String::from("Unable to create shader object"))?;

  context.attach_shader(&program, vert_shader);
  context.attach_shader(&program, frag_shader);
  context.link_program(&program);

  if context
    .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
    .as_bool()
    .unwrap_or(false)
  {
    Ok(program)
  } else {
    Err(
      context
        .get_program_info_log(&program)
        .unwrap_or_else(|| String::from("Unknown error creating program object")),
    )
  }
}
