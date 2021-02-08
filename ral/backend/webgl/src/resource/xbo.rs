use crate::{WebGLRenderer, WebGLVertexBuffer};
use rendiation_ral::VertexBufferLayout;
use web_sys::*;

// UBO
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

// VBO
#[allow(clippy::transmute_ptr_to_ptr)]
impl WebGLRenderer {
  pub fn create_vertex_buffer(
    &self,
    data: &[u8],
    layout: VertexBufferLayout<'static>,
  ) -> WebGLVertexBuffer {
    let buffer = self
      .gl
      .create_buffer()
      .ok_or("failed to create buffer")
      .unwrap();
    self
      .gl
      .bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));
    unsafe {
      let transmuted = std::mem::transmute::<&[u8], &[f32]>(data);
      let vert_array = js_sys::Float32Array::view(transmuted);
      self.gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &vert_array,
        WebGl2RenderingContext::STATIC_DRAW,
      );
    };
    WebGLVertexBuffer { buffer, layout }
  }
  pub fn dispose_vertex_buffer(&self, buffer: WebGLVertexBuffer) {
    self.gl.delete_buffer(Some(&buffer.buffer));
  }
}

// IBO
#[allow(clippy::transmute_ptr_to_ptr)]
impl WebGLRenderer {
  pub fn create_index_buffer(&self, data: &[u8]) -> WebGlBuffer {
    let buffer = self
      .gl
      .create_buffer()
      .ok_or("failed to create buffer")
      .unwrap();
    self
      .gl
      .bind_buffer(WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER, Some(&buffer));
    unsafe {
      // unsafe for transmute and avoid allocation(cause heap grow and move in wasm)
      let transmuted = std::mem::transmute::<&[u8], &[u16]>(data);
      let vert_array = js_sys::Uint16Array::view(transmuted);
      self.gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER,
        &vert_array,
        WebGl2RenderingContext::STATIC_DRAW,
      );
    };
    buffer
  }
  pub fn dispose_index_buffer(&self, buffer: WebGlBuffer) {
    self.gl.delete_buffer(Some(&buffer));
  }
}
