use crate::WebGLRenderer;
use rendiation_ral::*;
use web_sys::*;

pub struct WebGLVertexBuffer {
  pub buffer: WebGlBuffer,
  pub layout: VertexBufferLayout<'static>,
  // todo use small vec opt
  // todo optional VAO
}

pub fn format_to_webgl_data_type(d: VertexFormat) -> u32 {
  use VertexFormat::*;
  match d {
    Float | Float2 | Float3 | Float4 => WebGl2RenderingContext::FLOAT,
    // Float2 => WebGl2RenderingContext::UNSIGNED_SHORT,
    // Float3 => WebGl2RenderingContext::SHORT,
    // Float4 => WebGl2RenderingContext::BYTE,
    _ => panic!("unsupported"),
  }
}

pub fn format_to_webgl_data_size(d: VertexFormat) -> i32 {
  use VertexFormat::*;
  match d {
    Float => 1,
    Float2 => 2,
    Float3 => 3,
    Float4 => 4,
    _ => panic!("unsupported"),
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
    vertex_buffer.layout.attributes.iter().for_each(|b| {
      // consider avoid conversion every time
      self.gl.vertex_attrib_pointer_with_i32(
        index as u32,
        format_to_webgl_data_size(b.format),
        format_to_webgl_data_type(b.format),
        false,
        vertex_buffer.layout.array_stride as i32,
        b.offset as i32,
      );
    });

    self.gl.enable_vertex_attrib_array(index as u32);
    self.attribute_states.enable(index as usize, None);
  }
}
