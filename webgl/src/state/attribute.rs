use crate::WebGLRenderer;
use rendiation_ral::*;
use web_sys::*;

pub struct WebGLVertexBuffer {
  pub buffer: WebGlBuffer,
  pub layout: RALVertexBufferDescriptor,
  // todo use small vec opt
  // todo optional VAO
}

pub fn to_webgl(d: RALVertexAttributeDataType) -> u32 {
  use RALVertexAttributeDataType::*;
  match d {
    F32 => WebGl2RenderingContext::FLOAT,
    U16 => WebGl2RenderingContext::UNSIGNED_SHORT,
    I16 => WebGl2RenderingContext::SHORT,
    I8 => WebGl2RenderingContext::BYTE,
    U8 => WebGl2RenderingContext::UNSIGNED_BYTE,
  }
}

pub struct VertexEnableStates {
  current_generation: u64,
  slots: Vec<VertexEnabledStateSlotInfo>,
}

impl VertexEnableStates {
  pub fn new(max_attribute_count: usize) -> Self {
    Self {
      current_generation: 0,
      slots: vec![VertexEnabledStateSlotInfo::default(); max_attribute_count],
    }
  }
}

impl VertexEnableStates {
  pub fn prepare_new_bindings(&mut self) {
    self.current_generation += 1;
  }
  pub fn enable(&mut self, slot: usize, div: Option<u32>) {
    let slot = &mut self.slots[slot];
    slot.enabled = true;
    slot.divisor = div;
    slot.generation = self.current_generation;
  }
  pub fn disable_old_unused_bindings(&mut self, gl: &WebGl2RenderingContext) {
    self.slots.iter().enumerate().for_each(|(i, s)| {
      if s.generation != self.current_generation {
        gl.disable_vertex_attrib_array(i as u32);
      }
    })
  }
}

impl WebGLRenderer {
  pub fn disable_old_unused_bindings(&mut self) {
    self.attribute_states.disable_old_unused_bindings(&self.gl);
  }
}

#[derive(Copy, Clone)]
pub struct VertexEnabledStateSlotInfo {
  generation: u64,
  enabled: bool,
  divisor: Option<u32>,
}

impl Default for VertexEnabledStateSlotInfo {
  fn default() -> Self {
    Self {
      generation: 0,
      enabled: false,
      divisor: None,
    }
  }
}

impl WebGLRenderer {
  pub fn set_index_buffer(&self, buffer: Option<&WebGlBuffer>) {
    self
      .gl
      .bind_buffer(WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER, buffer)
  }

  pub fn set_vertex_buffer(&mut self, index: i32, vertex_buffer: &WebGLVertexBuffer) {
    self.gl.bind_buffer(
      WebGl2RenderingContext::ARRAY_BUFFER,
      Some(&vertex_buffer.buffer),
    );
    vertex_buffer.layout.attributes().iter().for_each(|b| {
      self.gl.vertex_attrib_pointer_with_i32(
        index as u32,
        b.size,
        to_webgl(b.data_type),
        false,
        vertex_buffer.layout.byte_stride,
        b.byte_offset,
      );
    });

    self.gl.enable_vertex_attrib_array(index as u32);
    self.attribute_states.enable(index as usize, None);
  }
}
