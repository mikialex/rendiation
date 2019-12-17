use rendiation_render_entity::*;
use crate::webgl::*;
use std::rc::Rc;
use web_sys::*;


impl WebGLRenderer {
  pub fn get_port(&self, shading: Rc<dyn Shading<Self>>) -> Result<Rc<dyn ShadingGPUPort<Self>>, String> {
    Ok(
      self
        .programs
        .borrow_mut()
        .entry(shading.clone())
        .or_insert_with(||{
          self.step_id.set(self.step_id.get() + 1);
          shading.make_gpu_port(self)
      })
        .clone(),
    )
  }
}

pub fn make_webgl_program(context: &WebGlRenderingContext, vertex_shader_str: &str, frag_shader_str: &str)
 -> Result<WebGlProgram, String>
{
  let vertex_shader = compile_shader(
    &context,
    WebGlRenderingContext::VERTEX_SHADER,
    vertex_shader_str,
  )?;
  let frag_shader = compile_shader(
    &context,
    WebGlRenderingContext::FRAGMENT_SHADER,
    frag_shader_str,
  )?;
  link_program(&context, &vertex_shader, &frag_shader)
}

fn compile_shader(
  context: &WebGlRenderingContext,
  shader_type: u32,
  source: &str,
) -> Result<WebGlShader, String> {
  let shader = context
    .create_shader(shader_type)
    .ok_or_else(|| String::from("Unable to create shader object"))?;
  context.shader_source(&shader, source);
  context.compile_shader(&shader);

  if context
    .get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
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
  context: &WebGlRenderingContext,
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
    .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
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
