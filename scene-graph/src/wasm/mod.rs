
use wasm_bindgen::prelude::*;
use crate::{WebGLBackend, Scene};

#[wasm_bindgen]
struct WASMScene{
  scene: Scene<WebGLBackend>
}
