use shadergraph::*;

use crate::{GPU2DTexture, GPUCommandEncoder};

// https://github.com/BabylonJS/Babylon.js/blob/d25bc29091/packages/dev/core/src/Engines/WebGPU/webgpuTextureHelper.ts

/// Mipmap generation is not supported in webgpu api for now, at least in mvp as far as i known.
/// It's also useful to provide customizable reducer / gen method for proper usage.
///
pub struct Mipmap2DGenerator {
  pub reducer: Box<dyn Mipmap2dReducer>,
}

impl Mipmap2DGenerator {
  pub fn generate(&self, encoder: &GPUCommandEncoder, texture: &GPU2DTexture) {}
}

pub trait Mipmap2dReducer {
  fn reduce(&self, input: Node<ShaderTexture2D>, range: Node<Vec4<f32>>) -> Node<f32>;
}
