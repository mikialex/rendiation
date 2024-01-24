mod cube;
mod d2;
mod pair;
mod sampler;

use std::task::Context;

pub use cube::*;
pub use d2::*;
pub use pair::*;
use rendiation_scene_core::*;
use rendiation_webgpu::*;
pub use sampler::*;

pub struct GPUTextureResourceSystem {
  d2: Box<dyn ReactiveCollection<AllocIdx<SceneTexture2DType>, GPU2DTextureView>>,
  cube: Box<dyn ReactiveCollection<AllocIdx<SceneTextureCubeImpl>, GPUCubeTextureView>>,
}

pub struct GPUTextureResourceGetter {
  pub d2: Box<dyn VirtualCollection<AllocIdx<SceneTexture2DType>, GPU2DTextureView>>,
  pub cube: Box<dyn VirtualCollection<AllocIdx<SceneTextureCubeImpl>, GPUCubeTextureView>>,
}

impl GPUTextureResourceGetter {
  pub fn get_d2(&self, d2: &SceneTexture2DType) -> &GPU2DTextureView {
    todo!()
  }
  pub fn get_cube(&self, d2: &SceneTexture2DType) -> &GPUCubeTextureView {
    todo!()
  }
}

impl GPUTextureResourceSystem {
  pub fn poll_updates(&self, cx: &mut Context) -> GPUTextureResourceGetter {
    todo!()
  }
}
