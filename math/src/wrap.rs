#[wasm_bindgen]
#[derive(Debug, Copy, Clone)]
pub struct MatWrap {
  value: Mat4<f32>,
}

type Mat4f32 = MatWrap;

#[wasm_bindgen]
pub struct Test {
  pub m: Mat4f32,
}
