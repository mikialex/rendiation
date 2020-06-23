use crate::WebGLRenderer;
use web_sys::*;

impl WebGLRenderer {
  pub fn set_index_buffer(&self, buffer: Option<&WebGlBuffer>) {
    self
      .gl
      .bind_buffer(WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER, buffer)
  }

  pub fn set_vertex_buffer(&self, index: usize, vertex_buffer: &WebGLVertexBuffer) {
    vertex_buffer.attributes.iter().for_each(|a| {
      self
        .gl
        .bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&a.buffer));
      self.gl.vertex_attrib_pointer_with_i32(
        index as u32,
        a.descriptor.size,
        a.descriptor.data_type.to_webgl(),
        false,
        vertex_buffer.stride,
        a.descriptor.offset,
      );
      self.gl.enable_vertex_attrib_array(index as u32);
    })
  }
}

#[derive(Copy, Clone)]
pub struct WebGLAttributeTypeId(u32);

pub struct WebGLVertexAttributeBuffer {
  buffer: WebGlBuffer,
  input_id: WebGLAttributeTypeId,
  descriptor: WebGLVertexAttributeBufferDescriptor,
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
      Self::Float => WebGl2RenderingContext::FLOAT,
    }
  }
}

pub struct WebGLVertexBuffer {
  stride: i32,
  attributes: Vec<WebGLVertexAttributeBuffer>,
  // todo use smallvec opt
  // todo optional VAO
}
