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
pub struct PhysicalSpecularGlossinessMaterial<S: SceneContent> {
  pub albedo: Vec3<f32>,
  pub specular: Vec3<f32>,
  pub glossiness: f32,
  pub emissive: Vec3<f32>,
  pub albedo_texture: Option<Texture2DWithSamplingData<S>>,
  pub specular_texture: Option<Texture2DWithSamplingData<S>>,
  pub glossiness_texture: Option<Texture2DWithSamplingData<S>>,
  pub emissive_texture: Option<Texture2DWithSamplingData<S>>,
}

impl<S: SceneContent> Default for PhysicalSpecularGlossinessMaterial<S> {
  fn default() -> Self {
    Self {
      albedo: Vec3::one(),
      specular: Vec3::zero(),
      glossiness: 0.5,
      emissive: Vec3::zero(),
      albedo_texture: None,
      specular_texture: None,
      glossiness_texture: None,
      emissive_texture: None,
    }
  }
}

#[derive(Clone)]
pub struct PhysicalMetallicRoughnessMaterial<S: SceneContent> {
  /// in conductor case will act as specular color,
  /// in dielectric case will act as diffuse color,
  pub base_color: Vec3<f32>,
  pub roughness: f32,
  pub metallic: f32,
  pub reflectance: f32,
  pub emissive: Vec3<f32>,
  pub base_color_texture: Option<Texture2DWithSamplingData<S>>,
  pub metallic_roughness_texture: Option<Texture2DWithSamplingData<S>>,
  pub emissive_texture: Option<Texture2DWithSamplingData<S>>,
}

impl<S: SceneContent> Default for PhysicalMetallicRoughnessMaterial<S> {
  fn default() -> Self {
    Self {
      base_color: Vec3::one(),
      roughness: 0.5,
      metallic: 0.0,
      emissive: Vec3::zero(),
      base_color_texture: None,
      metallic_roughness_texture: None,
      emissive_texture: None,
      reflectance: 0.5,
    }
  }
}

#[derive(Clone)]
pub struct FlatMaterial {
  pub color: Vec4<f32>,
}
