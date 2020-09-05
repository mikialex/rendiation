use crate::WebGLRenderer;
use web_sys::*;

impl WebGLRenderer {
  pub fn create_uniform_buffer(&self, data: &[u8]) -> WebGlBuffer {
    let gl = &self.gl;
    let buffer = gl
      .create_buffer()
      .ok_or("failed to create ubo buffer")
      .unwrap();
    gl.bind_buffer(WebGl2RenderingContext::UNIFORM_BUFFER, Some(&buffer));

    gl.buffer_data_with_u8_array_and_src_offset(
      WebGl2RenderingContext::UNIFORM_BUFFER,
      data,
      WebGl2RenderingContext::STATIC_DRAW,
      0,
    );
    buffer
  }

  pub fn delete_uniform_buffer(&self, buffer: WebGlBuffer) {
    self.gl.delete_buffer(Some(&buffer));
  }
}
