use crate::{CALVertexBufferLayout, WebGLRenderer, CALAttributeTypeId};
use web_sys::*;

impl WebGLRenderer {
  pub fn set_index_buffer(&self, buffer: Option<&WebGlBuffer>) {
    self
      .gl
      .bind_buffer(WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER, buffer)
  }

  pub fn set_vertex_buffer(&self, index: u32, vertex_buffer: &WebGLVertexBuffer) {
    // todo support interleave buffer;
    self
      .gl
      .bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&vertex_buffer.buffer));
      todo!()
    // self.gl.vertex_attrib_pointer_with_i32(
    //   index as u32,
    //   vertex_buffer.layout.stride,
    //   vertex_buffer.layout.data_type.to_webgl(),
    //   false,
    //   vertex_buffer.stride,
    //   vertex_buffer.descriptor.offset,
    // );
    // self.gl.enable_vertex_attrib_array(index as u32);
  }
}

pub struct WebGLVertexBuffer {
  pub input_id: CALAttributeTypeId,
  pub buffer: WebGlBuffer,
  pub layout: CALVertexBufferLayout,
  // todo use smallvec opt
  // todo optional VAO
}

