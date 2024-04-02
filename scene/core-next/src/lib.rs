use database::*;
use rendiation_algebra::*;

pub fn register_scene_core_data_model() {
  register_scene_data_model();
}

declare_entity!(SceneEntity);
// declare_component!(
//   SceneBackgroundComponent,
//   SceneEntity,
//   Option<SceneBackGround>
// );
declare_foreign_key!(SceneRootNodeForeignKey, SceneEntity, SceneNodeEntity);

pub fn register_scene_data_model() {
  global_database()
    .declare_entity::<SceneEntity>()
    // .declare_component::<SceneBackgroundComponent>()
    .declare_foreign_key::<SceneRootNodeForeignKey>();
}

declare_entity!(SceneNodeEntity);
declare_component!(SceneNodeLocalMatrixComponent, SceneNodeEntity, Mat4<f32>);
declare_component!(SceneNodeVisibleComponent, SceneNodeEntity, bool);

declare_entity!(PbrSGMaterialEntity);
declare_component!(PbrSGMaterialAlbedoComponent, PbrSGMaterialEntity, Vec3<f32>);
declare_component!(
  PbrSGMaterialSpecularComponent,
  PbrSGMaterialEntity,
  Vec3<f32>
);
declare_component!(PbrSGMaterialGlossinessComponent, PbrSGMaterialEntity, f32);
declare_component!(
  PbrSGMaterialEmissiveComponent,
  PbrSGMaterialEntity,
  Vec4<f32>
);
declare_component!(PbrSGMaterialAlphaComponent, PbrSGMaterialEntity, f32);
// declare_component!(
//   PbrSGMaterialAlphaModeComponent,
//   PbrSGMaterialEntity,
//   AlphaMode
// );

declare_entity_associated!(PbrSGMaterialAlbedoTex, PbrSGMaterialEntity);
impl TextureWithSamplingForeignKeys for PbrSGMaterialAlbedoTex {}
declare_entity_associated!(PbrSGMaterialSpecularTex, PbrSGMaterialEntity);
impl TextureWithSamplingForeignKeys for PbrSGMaterialSpecularTex {}
declare_entity_associated!(PbrSGMaterialGlossinessTex, PbrSGMaterialEntity);
impl TextureWithSamplingForeignKeys for PbrSGMaterialGlossinessTex {}
declare_entity_associated!(PbrSGMaterialEmissiveTex, PbrSGMaterialEntity);
impl TextureWithSamplingForeignKeys for PbrSGMaterialEmissiveTex {}

declare_foreign_key!(
  PbrSGMaterialNormalTexForeignKey,
  PbrSGMaterialEntity,
  SceneTexture2dEntity
);

pub fn register_pbr_material_data_model() {
  let ecg = global_database()
    .declare_entity::<PbrSGMaterialEntity>()
    .declare_component::<PbrSGMaterialAlbedoComponent>()
    .declare_component::<PbrSGMaterialGlossinessComponent>()
    .declare_component::<PbrSGMaterialAlphaComponent>();

  let ecg = register_texture_with_sampling::<PbrSGMaterialAlbedoTex>(ecg);
  let ecg = register_texture_with_sampling::<PbrSGMaterialSpecularTex>(ecg);
  let ecg = register_texture_with_sampling::<PbrSGMaterialGlossinessTex>(ecg);
  let _ecg = register_texture_with_sampling::<PbrSGMaterialEmissiveTex>(ecg);
}

declare_entity!(SceneTexture2dEntity);
declare_entity!(SceneSamplerEntity);

pub trait TextureWithSamplingForeignKeys: EntityAssociateSemantic {}

pub struct SceneTexture2dRefOf<T>(T);
impl<T: TextureWithSamplingForeignKeys> EntityAssociateSemantic for SceneTexture2dRefOf<T> {
  type Entity = T::Entity;
}
impl<T: TextureWithSamplingForeignKeys> ComponentSemantic for SceneTexture2dRefOf<T> {
  type Data = Option<u32>;
}
impl<T: TextureWithSamplingForeignKeys> ForeignKeySemantic for SceneTexture2dRefOf<T> {
  type ForeignEntity = SceneTexture2dEntity;
}

pub struct SceneSamplerRefOf<T>(T);
impl<T: TextureWithSamplingForeignKeys> EntityAssociateSemantic for SceneSamplerRefOf<T> {
  type Entity = T::Entity;
}
impl<T: TextureWithSamplingForeignKeys> ComponentSemantic for SceneSamplerRefOf<T> {
  type Data = Option<u32>;
}
impl<T: TextureWithSamplingForeignKeys> ForeignKeySemantic for SceneSamplerRefOf<T> {
  type ForeignEntity = SceneSamplerEntity;
}

pub fn register_texture_with_sampling<T: TextureWithSamplingForeignKeys>(
  ecg: EntityComponentGroupTyped<T::Entity>,
) -> EntityComponentGroupTyped<T::Entity> {
  ecg
    .declare_foreign_key::<SceneTexture2dRefOf<T>>()
    .declare_foreign_key::<SceneSamplerRefOf<T>>()
}
