use crate::{TextureSlotStates, VertexEnableStates};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::*;

#[wasm_bindgen]
pub struct WebGLRenderer {
  #[wasm_bindgen(skip)]
  pub canvas: HtmlCanvasElement,

  #[wasm_bindgen(skip)]
  pub gl: WebGl2RenderingContext,

  #[wasm_bindgen(skip)]
  pub capacities: WebGLCapabilities,

  #[wasm_bindgen(skip)]
  pub attribute_states: VertexEnableStates,

  #[wasm_bindgen(skip)]
  pub texture_slot_states: TextureSlotStates,
}

#[wasm_bindgen]
impl WebGLRenderer {
  #[wasm_bindgen(constructor)]
  pub fn new(canvas: HtmlCanvasElement) -> Self {
    let gl = canvas
      .get_context("webgl2")
      .unwrap()
      .unwrap()
      .dyn_into::<WebGl2RenderingContext>()
      .unwrap();

    let capacities = WebGLCapabilities::new(&gl);
    let attribute_states = VertexEnableStates::new(capacities.max_attribute_count as usize);
    let texture_slot_states = TextureSlotStates::new(capacities.max_combined_texture_image_units);

    Self {
      canvas,
      gl,
      capacities,
      attribute_states,
      texture_slot_states,
    }
  }
}

pub struct WebGLCapabilities {
  pub max_combined_texture_image_units: u32,
  pub max_attribute_count: u32,
}

impl WebGLCapabilities {
  pub fn new(gl: &WebGl2RenderingContext) -> Self {
    fn get_parameter_u32(gl: &WebGl2RenderingContext, parameter: u32) -> u32 {
      gl.get_parameter(parameter)
        .unwrap()
        .as_f64()
        .map(|v| v as u32)
        // Errors will be caught by the browser or through `get_error`
        // so return a default instead
        .unwrap_or(0)
    }

    Self {
      max_combined_texture_image_units: get_parameter_u32(
        gl,
        WebGl2RenderingContext::MAX_COMBINED_TEXTURE_IMAGE_UNITS,
      ),
      max_attribute_count: get_parameter_u32(gl, WebGl2RenderingContext::MAX_VERTEX_ATTRIBS),
    }
  }
}
