use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[repr(C)]
pub struct Vec2F32WASM {
  pub x: f32,
  pub y: f32,
}

#[wasm_bindgen]
#[repr(C)]
pub struct Vec3F32WASM {
  pub x: f32,
  pub y: f32,
  pub z: f32,
}

#[wasm_bindgen]
#[repr(C)]
pub struct Vec4F32WASM {
  pub x: f32,
  pub y: f32,
  pub z: f32,
  pub w: f32,
}
