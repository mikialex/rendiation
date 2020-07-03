use crate::VertexEnableStates;
use web_sys::*;

pub struct WebGLRenderer {
  pub gl: WebGl2RenderingContext,
  pub attribute_states: VertexEnableStates,
}
