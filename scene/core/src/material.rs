pub use flat_material::*;

use crate::*;
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
      .declare_component::<PbrSGMaterialGlossinessComponent>()
      .declare_component::<PbrSGMaterialEmissiveComponent>()
      .declare_component::<PbrSGMaterialAlphaComponent>();

    let ecg = register_texture_with_sampling::<PbrSGMaterialAlbedoTex>(ecg);
    let ecg = register_texture_with_sampling::<PbrSGMaterialSpecularTex>(ecg);
    let ecg = register_texture_with_sampling::<PbrSGMaterialGlossinessTex>(ecg);
    let ecg = register_texture_with_sampling::<PbrSGMaterialEmissiveTex>(ecg);
    register_normal::<PbrSGMaterialNormalInfo>(ecg);
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
      .declare_component::<PbrMRMaterialRoughnessComponent>()
      .declare_component::<PbrMRMaterialMetallicComponent>()
      .declare_component::<PbrMRMaterialEmissiveComponent>()
      .declare_component::<PbrMRMaterialAlphaComponent>();

    let ecg = register_texture_with_sampling::<PbrMRMaterialBaseColorTex>(ecg);
    let ecg = register_texture_with_sampling::<PbrMRMaterialMetallicRoughnessTex>(ecg);
    let ecg = register_texture_with_sampling::<PbrMRMaterialEmissiveTex>(ecg);
    register_normal::<PbrMRMaterialNormalInfo>(ecg);
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
