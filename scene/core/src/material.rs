use rendiation_algebra::*;
use rendiation_texture::TextureSampler;

use crate::*;

#[derive(Clone)]
pub struct TextureWithSamplingData<T> {
  pub texture: T,
  pub sampler: TextureSampler,
}

pub type Texture2DWithSamplingData<S> = TextureWithSamplingData<SceneTexture2D<S>>;

#[derive(Clone)]
pub struct PhysicalMaterial<S: SceneContent> {
  pub albedo: Vec3<f32>,
  pub specular: Vec3<f32>,
  pub glossiness: f32,
  pub albedo_texture: Option<Texture2DWithSamplingData<S>>,
  pub specular_texture: Option<Texture2DWithSamplingData<S>>,
  pub glossiness_texture: Option<Texture2DWithSamplingData<S>>,
}

#[derive(Clone)]
pub struct FlatMaterial {
  pub color: Vec4<f32>,
}
