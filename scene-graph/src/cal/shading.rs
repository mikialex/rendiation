use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct SceneShadingDescriptor {
  vertex_shader_str: String, // new sal(shading abstraction layer) is in design, assume shader just works
  frag_shader_str: String,
  pub rasterization_state: RasterizationState,
  // .. blend state stuff
  // .. target state stuff,

  // some think?
  // in opengl like backend, blend/target state is dynamically set on the ctx, target state is not be used at all.
  // in webgpu like backend, two mode:
  // 1. these state should explicitly and correctly provided and not perform runtime check, panic when not ok
  // 2. these state hashing to choose cached pso or create new in runtime, extra overhead and always ok.
  // but where should the strategy impl
}


impl SceneShadingDescriptor {  
  pub fn vertex_shader_str(&self) -> &str {
  &self.vertex_shader_str
}

pub fn frag_shader_str(&self) -> &str {
  &self.frag_shader_str
}
}

#[wasm_bindgen]
impl SceneShadingDescriptor {
  #[wasm_bindgen]
  pub fn new(vertex_shader_str: &str, frag_shader_str: &str) -> Self {
    Self {
      vertex_shader_str: vertex_shader_str.to_owned(),
      frag_shader_str: frag_shader_str.to_owned(),
      rasterization_state: RasterizationState::default(),
    }
  }

  #[wasm_bindgen]
  pub fn vertex_shader_str_wasm(&self) -> String {
    self.vertex_shader_str.clone()
  }

  #[wasm_bindgen]
  pub fn frag_shader_str_wasm(&self) -> String {
    self.frag_shader_str.clone()
  }
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
#[derive(Copy, Clone, Debug)]
pub struct RasterizationState {
  pub front_face: FrontFace,
  pub cull_mode: CullMode,
  pub depth_bias: f32,
  pub depth_bias_slope_scale: f32,
  pub depth_bias_clamp: f32,
}

impl Default for RasterizationState {
  fn default() -> Self {
    Self {
      front_face: FrontFace::Ccw,
      cull_mode: CullMode::None,
      depth_bias: 0.0,
      depth_bias_slope_scale: 0.0,
      depth_bias_clamp: 0.0,
    }
  }
}
