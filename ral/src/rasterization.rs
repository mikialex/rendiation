use wasm_bindgen::prelude::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum PrimitiveTopology {
  PointList = 0,
  LineList = 1,
  LineStrip = 2,
  TriangleList = 3,
  TriangleStrip = 4,
}

#[repr(C)]
#[wasm_bindgen]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum CullMode {
  None = 0,
  Front = 1,
  Back = 2,
}

#[repr(C)]
#[wasm_bindgen]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum FrontFace {
  Ccw = 0,
  Cw = 1,
}

#[wasm_bindgen]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct RasterizationState {
  pub front_face: FrontFace,
  pub cull_mode: CullMode,
  // pub depth_bias: f32,
  // pub depth_bias_slope_scale: f32,
  // pub depth_bias_clamp: f32,
}

impl Default for RasterizationState {
  fn default() -> Self {
    Self {
      front_face: FrontFace::Ccw,
      cull_mode: CullMode::None,
      // depth_bias: 0.0,
      // depth_bias_slope_scale: 0.0,
      // depth_bias_clamp: 0.0,
    }
  }
}
