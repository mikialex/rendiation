use std::{any::Any, cell::RefCell};

use crate::{WebGL, WebGLRenderer};
use rendiation_ral::*;
use web_sys::*;

pub trait WebGLUniformUploadShaderInstance {
  fn upload_all(
    &mut self,
    renderer: &mut WebGLRenderer,
    resource_manager: &ResourceManager<WebGL>,
    handle_object: &dyn Any,
  );
}

pub trait WebGLUniformUploadShaderInstanceBuilder {
  fn create_uploader(
    &self,
    gl: &WebGl2RenderingContext,
    program: &WebGlProgram,
  ) -> Box<dyn WebGLUniformUploadShaderInstance>;
}

pub struct WebGLProgram {
  program: WebGlProgram,
  uniforms: RefCell<Box<dyn WebGLUniformUploadShaderInstance>>,
}

pub struct WebGLProgramBuildSource {
  pub glsl_vertex: String,
  pub glsl_fragment: String,
  pub uploader_creator: Box<dyn WebGLUniformUploadShaderInstanceBuilder>,
}

impl WebGLProgram {
  pub fn new(renderer: &mut WebGLRenderer, des: &WebGLProgramBuildSource) -> Self {
    let gl = &renderer.gl;
    let program = make_webgl_program(gl, &des.glsl_vertex, &des.glsl_fragment).unwrap();
    let uniforms = RefCell::new(des.uploader_creator.create_uploader(gl, &program));

    WebGLProgram { program, uniforms }
  }

  pub fn upload(
    &self,
    renderer: &mut WebGLRenderer,
    resource_manager: &ResourceManager<WebGL>,
    handle_object: &dyn Any,
  ) {
    self
      .uniforms
      .borrow_mut()
      .upload_all(renderer, resource_manager, handle_object)
  }

  pub fn program(&self) -> &WebGlProgram {
    &self.program
  }
}

impl WebGLRenderer {
  pub fn use_program(&mut self, p: &WebGLProgram) {
    self.gl.use_program(Some(&p.program))
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
