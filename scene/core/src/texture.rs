use rendiation_texture::TextureSampler;

use crate::SceneItemRef;

#[derive(Clone)]
pub struct TextureWithSamplingData<T> {
  pub texture: T,
  pub sampler: TextureSampler,
}

pub type Texture2DWithSamplingData = TextureWithSamplingData<SceneTexture2D>;

pub struct SceneTexture2D {
  source: usize,
}

pub type SceneTextureCube<S> = SceneItemRef<<S as SceneContent>::TextureCube>;
