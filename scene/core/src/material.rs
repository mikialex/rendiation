use rendiation_algebra::*;
use rendiation_texture::TextureSampler;

use crate::*;

/// The alpha rendering mode of a material.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum AlphaMode {
  /// The alpha value is ignored and the rendered output is fully opaque.
  Opaque,

  /// The rendered output is either fully opaque or fully transparent depending on
  /// the alpha value and the specified alpha cutoff value.
  Mask,

  /// The alpha value is used, to determine the transparency of the rendered output.
  /// The alpha cutoff value is ignored.
  Blend,
}

impl Default for AlphaMode {
  fn default() -> Self {
    Self::Opaque
  }
}

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
  pub alpha: f32,
  pub alpha_cutoff: f32,
  pub alpha_mode: AlphaMode,
  pub albedo_texture: Option<Texture2DWithSamplingData<S>>,
  pub specular_texture: Option<Texture2DWithSamplingData<S>>,
  pub glossiness_texture: Option<Texture2DWithSamplingData<S>>,
  pub emissive_texture: Option<Texture2DWithSamplingData<S>>,
  pub normal_texture: Option<NormalMapping<S>>,
}

#[derive(Clone)]
pub struct NormalMapping<S: SceneContent> {
  pub content: Texture2DWithSamplingData<S>,
  pub scale: f32,
}

impl<S: SceneContent> Default for PhysicalSpecularGlossinessMaterial<S> {
  fn default() -> Self {
    Self {
      albedo: Vec3::one(),
      specular: Vec3::zero(),
      glossiness: 0.5,
      emissive: Vec3::zero(),
      alpha: 1.0,
      alpha_cutoff: 1.0,
      alpha_mode: Default::default(),
      albedo_texture: None,
      specular_texture: None,
      glossiness_texture: None,
      emissive_texture: None,
      normal_texture: None,
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
  pub alpha: f32,
  pub alpha_cutoff: f32,
  pub alpha_mode: AlphaMode,
  pub base_color_texture: Option<Texture2DWithSamplingData<S>>,
  pub metallic_roughness_texture: Option<Texture2DWithSamplingData<S>>,
  pub emissive_texture: Option<Texture2DWithSamplingData<S>>,
  pub normal_texture: Option<NormalMapping<S>>,
}

impl<S: SceneContent> Default for PhysicalMetallicRoughnessMaterial<S> {
  fn default() -> Self {
    Self {
      base_color: Vec3::one(),
      roughness: 0.5,
      metallic: 0.0,
      alpha: 1.0,
      alpha_cutoff: 1.0,
      alpha_mode: Default::default(),
      emissive: Vec3::zero(),
      base_color_texture: None,
      metallic_roughness_texture: None,
      emissive_texture: None,
      reflectance: 0.5,
      normal_texture: None,
    }
  }
}

#[derive(Clone)]
pub struct FlatMaterial {
  pub color: Vec4<f32>,
}
