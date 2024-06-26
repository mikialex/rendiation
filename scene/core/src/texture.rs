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
declare_foreign_key!(
  SceneTextureCubeXPositiveFace,
  SceneTextureCubeEntity,
  SceneTexture2dEntity
);
declare_foreign_key!(
  SceneTextureCubeYPositiveFace,
  SceneTextureCubeEntity,
  SceneTexture2dEntity
);
declare_foreign_key!(
  SceneTextureCubeZPositiveFace,
  SceneTextureCubeEntity,
  SceneTexture2dEntity
);
declare_foreign_key!(
  SceneTextureCubeXNegativeFace,
  SceneTextureCubeEntity,
  SceneTexture2dEntity
);
declare_foreign_key!(
  SceneTextureCubeYNegativeFace,
  SceneTextureCubeEntity,
  SceneTexture2dEntity
);
declare_foreign_key!(
  SceneTextureCubeZNegativeFace,
  SceneTextureCubeEntity,
  SceneTexture2dEntity
);

pub fn register_scene_texture_cube_data_model() {
  global_database()
    .declare_entity::<SceneTextureCubeEntity>()
    .declare_foreign_key::<SceneTextureCubeXPositiveFace>()
    .declare_foreign_key::<SceneTextureCubeYPositiveFace>()
    .declare_foreign_key::<SceneTextureCubeZPositiveFace>()
    .declare_foreign_key::<SceneTextureCubeXNegativeFace>()
    .declare_foreign_key::<SceneTextureCubeYNegativeFace>()
    .declare_foreign_key::<SceneTextureCubeZNegativeFace>();
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
  type Data = ForeignKeyComponentData;
}
impl<T: TextureWithSamplingForeignKeys> ForeignKeySemantic for SceneTexture2dRefOf<T> {
  type ForeignEntity = SceneTexture2dEntity;
}

pub struct SceneSamplerRefOf<T>(T);
impl<T: TextureWithSamplingForeignKeys> EntityAssociateSemantic for SceneSamplerRefOf<T> {
  type Entity = T::Entity;
}
impl<T: TextureWithSamplingForeignKeys> ComponentSemantic for SceneSamplerRefOf<T> {
  type Data = ForeignKeyComponentData;
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

#[derive(Clone)]
pub struct Texture2DWithSamplingDataView {
  pub texture: EntityHandle<SceneTexture2dEntity>,
  pub sampler: EntityHandle<SceneSamplerEntity>,
}

impl Texture2DWithSamplingDataView {
  pub fn write<C, E>(self, writer: &mut EntityWriter<E>)
  where
    E: EntitySemantic,
    C: TextureWithSamplingForeignKeys,
    C: EntityAssociateSemantic<Entity = E>,
  {
    writer
      .component_value_writer::<SceneTexture2dRefOf<C>>(self.texture.some_handle())
      .component_value_writer::<SceneSamplerRefOf<C>>(self.sampler.some_handle());
  }
}

pub struct TexSamplerWriter<'a> {
  pub tex_writer: &'a mut EntityWriter<SceneTexture2dEntity>,
  pub sampler_writer: &'a mut EntityWriter<SceneSamplerEntity>,
}

impl<'a> TexSamplerWriter<'a> {
  pub fn write_tex_with_default_sampler(
    &mut self,
    texture: GPUBufferImage,
  ) -> Texture2DWithSamplingDataView {
    self.write(texture, TextureSampler::tri_linear_repeat())
  }
  pub fn write(
    &mut self,
    texture: GPUBufferImage,
    sampler: TextureSampler,
  ) -> Texture2DWithSamplingDataView {
    let texture = ExternalRefPtr::new(texture);

    let sampler = self
      .sampler_writer
      .component_value_writer::<SceneSamplerInfo>(sampler)
      .new_entity();

    let texture = self
      .tex_writer
      .component_value_writer::<SceneTexture2dEntityDirectContent>(texture.into())
      .new_entity();

    Texture2DWithSamplingDataView { texture, sampler }
  }
}
