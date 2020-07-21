use crate::VertexEnableStates;
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
    }
  }
}
