use std::fmt::Debug;

use rendiation_texture::{GPUBufferImage, TextureSampler};

use crate::*;

#[derive(Clone)]
pub struct TextureWithSamplingData<T> {
  pub texture: T,
  pub sampler: SharedIncrementalSignal<TextureSampler>,
}

impl<T: Clone + Send + Sync> SimpleIncremental for TextureWithSamplingData<T> {
  type Delta = Self;

  fn s_apply(&mut self, delta: Self::Delta) {
    *self = delta
  }

  fn s_expand(&self, mut cb: impl FnMut(Self::Delta)) {
    cb(self.clone())
  }
}

pub type Texture2DWithSamplingData = TextureWithSamplingData<SceneTexture2D>;

pub type SceneTexture2D = SharedIncrementalSignal<SceneTexture2DType>;

#[non_exhaustive]
#[derive(Clone)]
pub enum SceneTexture2DType {
  GPUBufferImage(GPUBufferImage),
  Foreign(Box<dyn AnyClone + Send + Sync>),
}

clone_self_incremental!(SceneTexture2DType);

impl Debug for SceneTexture2DType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("SceneTexture2DType")
  }
}

pub type SceneTextureCube = SharedIncrementalSignal<SceneTextureCubeImpl>;

#[derive(Clone)]
pub struct SceneTextureCubeImpl {
  pub faces: [SceneTexture2DType; 6],
}

clone_self_incremental!(SceneTextureCubeImpl);
