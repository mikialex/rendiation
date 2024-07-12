pub use flat_material::*;

use crate::*;

#[derive(Clone, Copy)]
pub enum SceneMaterialDataView {
  FlatMaterial(EntityHandle<FlatMaterialEntity>),
  PbrSGMaterial(EntityHandle<PbrSGMaterialEntity>),
  PbrMRMaterial(EntityHandle<PbrMRMaterialEntity>),
}

mod flat_material {
  use crate::*;
  declare_entity!(FlatMaterialEntity);
  declare_component!(
    FlatMaterialDisplayColorComponent,
    FlatMaterialEntity,
    Vec4<f32>,
    Vec4::one()
  );
  pub fn register_flat_material_data_model() {
    global_database()
      .declare_entity::<FlatMaterialEntity>()
      .declare_component::<FlatMaterialDisplayColorComponent>();
  }

  pub struct FlatMaterialDataView {
    pub color: Vec4<f32>,
  }
  impl EntityCustomWrite<FlatMaterialEntity> for FlatMaterialDataView {
    type Writer = EntityWriter<FlatMaterialEntity>;

    fn create_writer() -> Self::Writer {
      global_entity_of::<FlatMaterialEntity>().entity_writer()
    }

    fn write(self, writer: &mut Self::Writer) -> EntityHandle<FlatMaterialEntity> {
      writer
        .component_value_writer::<FlatMaterialDisplayColorComponent>(self.color)
        .new_entity()
    }
  }
}

pub use sg_material::*;
mod sg_material {
  use crate::*;
  declare_entity!(PbrSGMaterialEntity);
  declare_component!(
    PbrSGMaterialAlbedoComponent,
    PbrSGMaterialEntity,
    Vec3<f32>,
    Vec3::one()
  );
  declare_component!(
    PbrSGMaterialSpecularComponent,
    PbrSGMaterialEntity,
    Vec3<f32>,
    Vec3::zero()
  );
  declare_component!(
    PbrSGMaterialGlossinessComponent,
    PbrSGMaterialEntity,
    f32,
    0.5
  );
  declare_component!(
    PbrSGMaterialEmissiveComponent,
    PbrSGMaterialEntity,
    Vec3<f32>,
    Vec3::zero()
  );
  declare_component!(PbrSGMaterialAlphaComponent, PbrSGMaterialEntity, f32);
  declare_component!(
    PbrSGMaterialAlphaModeComponent,
    PbrSGMaterialEntity,
    AlphaMode
  );

  declare_entity_associated!(PbrSGMaterialAlbedoTex, PbrSGMaterialEntity);
  impl TextureWithSamplingForeignKeys for PbrSGMaterialAlbedoTex {}
  declare_entity_associated!(PbrSGMaterialSpecularTex, PbrSGMaterialEntity);
  impl TextureWithSamplingForeignKeys for PbrSGMaterialSpecularTex {}
  declare_entity_associated!(PbrSGMaterialGlossinessTex, PbrSGMaterialEntity);
  impl TextureWithSamplingForeignKeys for PbrSGMaterialGlossinessTex {}
  declare_entity_associated!(PbrSGMaterialEmissiveTex, PbrSGMaterialEntity);
  impl TextureWithSamplingForeignKeys for PbrSGMaterialEmissiveTex {}
  declare_entity_associated!(PbrSGMaterialNormalInfo, PbrSGMaterialEntity);
  impl NormalInfoSemantic for PbrSGMaterialNormalInfo {}

  pub fn register_pbr_sg_material_data_model() {
    let ecg = global_database()
      .declare_entity::<PbrSGMaterialEntity>()
      .declare_component::<PbrSGMaterialAlbedoComponent>()
      .declare_component::<PbrSGMaterialSpecularComponent>()
      .declare_component::<PbrSGMaterialGlossinessComponent>()
      .declare_component::<PbrSGMaterialEmissiveComponent>()
      .declare_component::<PbrSGMaterialAlphaComponent>();

    let ecg = register_texture_with_sampling::<PbrSGMaterialAlbedoTex>(ecg);
    let ecg = register_texture_with_sampling::<PbrSGMaterialSpecularTex>(ecg);
    let ecg = register_texture_with_sampling::<PbrSGMaterialGlossinessTex>(ecg);
    let ecg = register_texture_with_sampling::<PbrSGMaterialEmissiveTex>(ecg);
    register_normal::<PbrSGMaterialNormalInfo>(ecg);
  }

  #[derive(Clone)]
  pub struct PhysicalSpecularGlossinessMaterialDataView {
    pub albedo: Vec3<f32>,
    pub specular: Vec3<f32>,
    pub glossiness: f32,
    pub emissive: Vec3<f32>,
    pub alpha: f32,
    pub alpha_cutoff: f32,
    pub alpha_mode: AlphaMode,
    pub albedo_texture: Option<Texture2DWithSamplingDataView>,
    pub specular_texture: Option<Texture2DWithSamplingDataView>,
    pub glossiness_texture: Option<Texture2DWithSamplingDataView>,
    pub emissive_texture: Option<Texture2DWithSamplingDataView>,
    pub normal_texture: Option<NormalMappingDataView>,
  }

  impl Default for PhysicalSpecularGlossinessMaterialDataView {
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

  impl PhysicalSpecularGlossinessMaterialDataView {
    pub fn write(
      self,
      writer: &mut EntityWriter<PbrSGMaterialEntity>,
    ) -> EntityHandle<PbrSGMaterialEntity> {
      writer
        .component_value_writer::<PbrSGMaterialAlbedoComponent>(self.albedo)
        .component_value_writer::<PbrSGMaterialAlphaModeComponent>(self.alpha_mode)
        .component_value_writer::<PbrSGMaterialSpecularComponent>(self.specular)
        .component_value_writer::<PbrSGMaterialGlossinessComponent>(self.glossiness)
        .component_value_writer::<PbrSGMaterialEmissiveComponent>(self.emissive)
        .component_value_writer::<PbrSGMaterialAlphaComponent>(self.alpha);

      if let Some(t) = self.albedo_texture {
        t.write::<PbrSGMaterialAlbedoTex, _>(writer)
      }

      if let Some(t) = self.specular_texture {
        t.write::<PbrSGMaterialSpecularTex, _>(writer)
      }

      if let Some(t) = self.glossiness_texture {
        t.write::<PbrSGMaterialGlossinessTex, _>(writer)
      }

      if let Some(t) = self.emissive_texture {
        t.write::<PbrSGMaterialEmissiveTex, _>(writer)
      }

      // todo normal map

      writer.new_entity()
    }
  }
}

pub use mr_material::*;
mod mr_material {
  use crate::*;
  declare_entity!(PbrMRMaterialEntity);
  declare_component!(
    PbrMRMaterialBaseColorComponent,
    PbrMRMaterialEntity,
    Vec3<f32>,
    Vec3::one()
  );
  declare_component!(
    PbrMRMaterialMetallicComponent,
    PbrMRMaterialEntity,
    f32,
    0.0
  );
  declare_component!(
    PbrMRMaterialRoughnessComponent,
    PbrMRMaterialEntity,
    f32,
    0.5
  );
  declare_component!(
    PbrMRMaterialEmissiveComponent,
    PbrMRMaterialEntity,
    Vec3<f32>,
    Vec3::zero()
  );
  declare_component!(PbrMRMaterialAlphaComponent, PbrMRMaterialEntity, f32);
  declare_component!(
    PbrMRMaterialAlphaModeComponent,
    PbrMRMaterialEntity,
    AlphaMode
  );

  declare_entity_associated!(PbrMRMaterialBaseColorTex, PbrMRMaterialEntity);
  impl TextureWithSamplingForeignKeys for PbrMRMaterialBaseColorTex {}
  declare_entity_associated!(PbrMRMaterialMetallicRoughnessTex, PbrMRMaterialEntity);
  impl TextureWithSamplingForeignKeys for PbrMRMaterialMetallicRoughnessTex {}
  declare_entity_associated!(PbrMRMaterialEmissiveTex, PbrMRMaterialEntity);
  impl TextureWithSamplingForeignKeys for PbrMRMaterialEmissiveTex {}
  declare_entity_associated!(PbrMRMaterialNormalInfo, PbrMRMaterialEntity);
  impl NormalInfoSemantic for PbrMRMaterialNormalInfo {}

  pub fn register_pbr_mr_material_data_model() {
    let ecg = global_database()
      .declare_entity::<PbrMRMaterialEntity>()
      .declare_component::<PbrMRMaterialBaseColorComponent>()
      .declare_component::<PbrMRMaterialRoughnessComponent>()
      .declare_component::<PbrMRMaterialMetallicComponent>()
      .declare_component::<PbrMRMaterialEmissiveComponent>()
      .declare_component::<PbrMRMaterialAlphaComponent>()
      .declare_component::<PbrMRMaterialAlphaModeComponent>();

    let ecg = register_texture_with_sampling::<PbrMRMaterialBaseColorTex>(ecg);
    let ecg = register_texture_with_sampling::<PbrMRMaterialMetallicRoughnessTex>(ecg);
    let ecg = register_texture_with_sampling::<PbrMRMaterialEmissiveTex>(ecg);
    register_normal::<PbrMRMaterialNormalInfo>(ecg);
  }

  #[derive(Clone)]
  pub struct PhysicalMetallicRoughnessMaterialDataView {
    /// in conductor case will act as specular color,
    /// in dielectric case will act as diffuse color,
    /// which is decided by metallic property
    pub base_color: Vec3<f32>,
    pub roughness: f32,
    pub metallic: f32,
    pub reflectance: f32,
    pub emissive: Vec3<f32>,
    pub alpha: f32,
    pub alpha_cutoff: f32,
    pub alpha_mode: AlphaMode,
    pub base_color_texture: Option<Texture2DWithSamplingDataView>,
    pub metallic_roughness_texture: Option<Texture2DWithSamplingDataView>,
    pub emissive_texture: Option<Texture2DWithSamplingDataView>,
    pub normal_texture: Option<NormalMappingDataView>,
  }

  impl Default for PhysicalMetallicRoughnessMaterialDataView {
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

  impl PhysicalMetallicRoughnessMaterialDataView {
    pub fn write(
      self,
      writer: &mut EntityWriter<PbrMRMaterialEntity>,
    ) -> EntityHandle<PbrMRMaterialEntity> {
      writer
        .component_value_writer::<PbrMRMaterialBaseColorComponent>(self.base_color)
        .component_value_writer::<PbrMRMaterialRoughnessComponent>(self.roughness)
        .component_value_writer::<PbrMRMaterialMetallicComponent>(self.metallic)
        .component_value_writer::<PbrMRMaterialEmissiveComponent>(self.emissive)
        .component_value_writer::<PbrMRMaterialAlphaComponent>(self.alpha);

      if let Some(t) = self.base_color_texture {
        t.write::<PbrMRMaterialBaseColorTex, _>(writer)
      }

      if let Some(t) = self.metallic_roughness_texture {
        t.write::<PbrMRMaterialMetallicRoughnessTex, _>(writer)
      }

      if let Some(t) = self.emissive_texture {
        t.write::<PbrMRMaterialEmissiveTex, _>(writer)
      }

      // todo normal map

      writer.new_entity()
    }
  }
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

impl Default for AlphaMode {
  fn default() -> Self {
    Self::Opaque
  }
}

pub trait NormalInfoSemantic: EntityAssociateSemantic {}
pub struct NormalScaleOf<T>(T);
impl<T: NormalInfoSemantic> EntityAssociateSemantic for NormalScaleOf<T> {
  type Entity = T::Entity;
}
impl<T: NormalInfoSemantic> ComponentSemantic for NormalScaleOf<T> {
  type Data = f32;
  fn default_override() -> Self::Data {
    1.0
  }
}
pub struct NormalTexSamplerOf<T>(T);
impl<T: NormalInfoSemantic> EntityAssociateSemantic for NormalTexSamplerOf<T> {
  type Entity = T::Entity;
}
impl<T: NormalInfoSemantic> TextureWithSamplingForeignKeys for NormalTexSamplerOf<T> {}
pub type NormalTexOf<T> = SceneTexture2dRefOf<NormalTexSamplerOf<T>>;
pub type NormalSamplerOf<T> = SceneSamplerRefOf<NormalTexSamplerOf<T>>;

pub fn register_normal<T: NormalInfoSemantic>(
  ecg: EntityComponentGroupTyped<T::Entity>,
) -> EntityComponentGroupTyped<T::Entity> {
  let ecg = register_texture_with_sampling::<NormalTexSamplerOf<T>>(ecg);
  ecg.declare_component::<NormalScaleOf<T>>()
}

#[derive(Clone)]
pub struct NormalMappingDataView {
  pub content: Texture2DWithSamplingDataView,
  pub scale: f32,
}
