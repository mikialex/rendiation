use rendiation_texture::TextureSampler;

use crate::SceneItemRef;

#[derive(Clone)]
pub struct TextureWithSamplingData<T> {
  pub texture: T,
  pub sampler: TextureSampler,
}

pub type Texture2DWithSamplingData = TextureWithSamplingData<SceneTexture2D>;

pub type SceneTexture2D = SceneItemRef<SceneTexture2DImpl>;
pub struct SceneTexture2DImpl {
  source: usize,
}

pub type SceneTextureCube = SceneItemRef<SceneTextureCubeImpl>;
pub struct SceneTextureCubeImpl {
  source: usize,
}
