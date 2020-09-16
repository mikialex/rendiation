use crate::{SceneShaderDescriptor, RasterizationState};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct SceneShadingDescriptor {
  #[wasm_bindgen(skip)]
  pub shader_descriptor: SceneShaderDescriptor,
  pub rasterization_state: RasterizationState,
  // primitive_topology: wgpu::PrimitiveTopology,
  // .. blend state stuff
  // .. target state stuff,

  // some think?
  // in opengl like backend, blend/target state is dynamically set on the ctx, target state is not be used at all.
  // in webgpu like backend, two mode:
  // 1. these state should explicitly and correctly provided and not perform runtime check, panic when not ok
  // 2. these state hashing to choose cached pso or create new in runtime, extra overhead and always ok.
  // but where should the strategy impl
}

#[wasm_bindgen]
impl SceneShadingDescriptor {
  #[wasm_bindgen(constructor)]
  pub fn new(shader_descriptor: SceneShaderDescriptor) -> Self {
    Self {
      shader_descriptor,
      rasterization_state: RasterizationState::default(),
    }
  }
}

