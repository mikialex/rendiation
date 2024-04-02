use std::fmt::Debug;

use rendiation_texture::{GPUBufferImage, TextureSampler};

use crate::*;

#[derive(Clone, Derivative)]
#[derivative(Hash)]
pub struct TextureWithSamplingData<T> {
  pub texture: T,
  #[derivative(Hash(hash_with = "ptr_internal_hash"))]
  pub sampler: IncrementalSignalPtr<TextureSampler>,
}

fn ptr_internal_hash<T: IncrementalBase + Hash, H>(value: &IncrementalSignalPtr<T>, state: &mut H)
where
  H: std::hash::Hasher,
{
  value.read().hash(state)
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

pub type SceneTexture2D = IncrementalSignalPtr<SceneTexture2DType>;

#[derive(Clone)]
pub enum SceneTexture2DType {
  GPUBufferImage(GPUBufferImage),
  Foreign(ForeignObject),
}

clone_self_incremental!(SceneTexture2DType);

impl Debug for SceneTexture2DType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("SceneTexture2DType")
  }
}

pub type SceneTextureCube = IncrementalSignalPtr<SceneTextureCubeImpl>;

#[derive(Clone, Debug)]
pub struct SceneTextureCubeImpl {
  /// in: px, nx, py, ny, pz, nz order
  pub faces: [SceneTexture2DType; 6],
}

clone_self_incremental!(SceneTextureCubeImpl);
