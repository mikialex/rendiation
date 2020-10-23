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

#[rustfmt::skip]
#[wasm_bindgen]
#[repr(C)]
pub struct Mat4F32WASM {
	pub a1:f32, pub a2:f32, pub a3:f32, pub a4:f32,
	pub b1:f32, pub b2:f32, pub b3:f32, pub b4:f32,
	pub c1:f32, pub c2:f32, pub c3:f32, pub c4:f32,
	pub d1:f32, pub d2:f32, pub d3:f32, pub d4:f32,
}
