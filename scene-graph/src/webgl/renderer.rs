use crate::WebGLVertexBuffer;
use web_sys::*;

pub struct WebGLRenderer {
  pub gl: WebGl2RenderingContext,
}

impl WebGLRenderer {
  pub fn use_program(&mut self, p: &WebGlProgram) {
    self.gl.use_program(Some(p))
  }

  pub fn set_index_buffer(&self, buffer: Option<&WebGlBuffer>) {
    self
      .gl
      .bind_buffer(WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER, buffer)
  }

  pub fn set_vertex_buffer(&self, _index: usize, vertex_buffer: &WebGLVertexBuffer) {
    vertex_buffer.attributes.iter().for_each(|a| {
      self
        .gl
        .bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&a.buffer));
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
