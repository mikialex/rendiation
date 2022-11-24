use std::fmt::Debug;

use rendiation_algebra::*;
use rendiation_texture::{Texture2DBuffer, TextureSampler};

use crate::*;

#[derive(Clone)]
pub struct TextureWithSamplingData<T> {
  pub texture: T,
  pub sampler: TextureSampler,
}

pub type Texture2DWithSamplingData = TextureWithSamplingData<SceneTexture2D>;

pub type SceneTexture2D = SceneItemRef<SceneTexture2DType>;

#[non_exhaustive]
pub enum SceneTexture2DType {
  RGBAu8(Texture2DBuffer<Vec4<u8>>),
  RGBu8(Texture2DBuffer<Vec3<u8>>),
  RGBAf32(Texture2DBuffer<Vec4<f32>>),
  Foreign(Arc<dyn Any + Send + Sync>),
}

impl Debug for SceneTexture2DType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("SceneTexture2DType")
  }
}

pub type SceneTextureCube = SceneItemRef<SceneTextureCubeImpl>;
pub struct SceneTextureCubeImpl {
  pub faces: [SceneTexture2DType; 6],
}
