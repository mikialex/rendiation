use rendiation_texture::{GPUBufferImage, TextureSampler};

use crate::*;

declare_entity!(SceneTexture2dEntity);
declare_component!(
  SceneTexture2dEntityDirectContent,
  SceneTexture2dEntity,
  Option<ExternalRefPtr<GPUBufferImage>>
);
pub fn register_scene_texture2d_data_model() {
  global_database()
    .declare_entity::<SceneTexture2dEntity>()
    .declare_component::<SceneTexture2dEntityDirectContent>();
}

declare_entity!(SceneTextureCubeEntity);
declare_component!(
  SceneTextureCubeEntityDirectContent,
  SceneTextureCubeEntity,
  Option<ExternalRefPtr<GPUBufferImage>>
);
pub fn register_scene_texture_cube_data_model() {
  global_database()
    .declare_entity::<SceneTextureCubeEntity>()
    .declare_component::<SceneTextureCubeEntityDirectContent>();
}

declare_entity!(SceneSamplerEntity);
declare_component!(SceneSamplerInfo, SceneSamplerEntity, TextureSampler);
pub fn register_scene_sampler_data_model() {
  global_database()
    .declare_entity::<SceneSamplerEntity>()
    .declare_component::<SceneSamplerInfo>();
}

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
