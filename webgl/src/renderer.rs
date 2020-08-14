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
    Self {
      canvas,
      gl,
      attribute_states: VertexEnableStates::new(10), // todo!()
      texture_slot_states: TextureSlotStates::new(8),
    }
  }
}

pub struct WebGLCapabilities {
  pub max_combined_texture_image_units: u32,
}

impl WebGLCapabilities {
  pub fn new(_gl: &WebGl2RenderingContext) -> Self {
    todo!()
    // Self {
    //   max_combined_texture_image_units: gl
    //     .get_parameter(WebGl2RenderingContext::MAX_COMBINED_TEXTURE_IMAGE_UNITS)
    //     .unwrap()
    //     .dyn_into::<u32>()
    //     .unwrap(),
    // }
  }
}
