use crate::*;

#[derive(Clone)]
pub enum MaterialEnum {
  PhysicalSpecularGlossiness(IncrementalSignalPtr<PhysicalSpecularGlossinessMaterial>),
  PhysicalMetallicRoughness(IncrementalSignalPtr<PhysicalMetallicRoughnessMaterial>),
  Flat(IncrementalSignalPtr<FlatMaterial>),
  Foreign(ForeignObject),
}

impl MaterialEnum {
  pub fn guid(&self) -> Option<u64> {
    match self {
      Self::PhysicalSpecularGlossiness(m) => m.guid(),
      Self::PhysicalMetallicRoughness(m) => m.guid(),
      Self::Flat(m) => m.guid(),
      Self::Foreign(m) => get_dyn_trait_downcaster_static!(GlobalIdentified)
        .downcast_ref(m.as_ref().as_any())?
        .guid(),
    }
    .into()
  }
}

clone_self_incremental!(MaterialEnum);

pub fn register_core_material_features<T>()
where
  T: AsRef<dyn GlobalIdentified> + AsMut<dyn GlobalIdentified> + 'static,
{
  get_dyn_trait_downcaster_static!(GlobalIdentified).register::<T>()
}

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
clone_self_incremental!(AlphaMode);

impl Default for AlphaMode {
  fn default() -> Self {
    Self::Opaque
  }
}

#[derive(Clone, Incremental, Derivative)]
#[derivative(Hash)]
pub struct PhysicalSpecularGlossinessMaterial {
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub albedo: Vec3<f32>,
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub specular: Vec3<f32>,
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub glossiness: f32,
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub emissive: Vec3<f32>,
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub alpha: f32,
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub alpha_cutoff: f32,
  pub alpha_mode: AlphaMode,
  pub albedo_texture: Option<Texture2DWithSamplingData>,
  pub specular_texture: Option<Texture2DWithSamplingData>,
  pub glossiness_texture: Option<Texture2DWithSamplingData>,
  pub emissive_texture: Option<Texture2DWithSamplingData>,
  pub normal_texture: Option<NormalMapping>,
}

#[derive(Clone, Incremental, Derivative)]
#[derivative(Hash)]
pub struct NormalMapping {
  pub content: Texture2DWithSamplingData,
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub scale: f32,
}

impl Default for PhysicalSpecularGlossinessMaterial {
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

#[derive(Clone, Incremental, Derivative)]
#[derivative(Hash)]
pub struct PhysicalMetallicRoughnessMaterial {
  /// in conductor case will act as specular color,
  /// in dielectric case will act as diffuse color,
  /// which is decided by metallic property
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub base_color: Vec3<f32>,
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub roughness: f32,
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub metallic: f32,
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub reflectance: f32,
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub emissive: Vec3<f32>,
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub alpha: f32,
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub alpha_cutoff: f32,
  pub alpha_mode: AlphaMode,
  pub base_color_texture: Option<Texture2DWithSamplingData>,
  pub metallic_roughness_texture: Option<Texture2DWithSamplingData>,
  pub emissive_texture: Option<Texture2DWithSamplingData>,
  pub normal_texture: Option<NormalMapping>,
}

impl Default for PhysicalMetallicRoughnessMaterial {
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

#[derive(Clone, Incremental, Derivative)]
#[derivative(Hash)]
pub struct FlatMaterial {
  #[derivative(Hash(hash_with = "byte_hash"))]
  pub color: Vec4<f32>,
}
