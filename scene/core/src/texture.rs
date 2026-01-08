use crate::*;

declare_entity!(SceneTexture2dEntity);
pub type TextureDirectContentType = Option<ExternalRefPtr<MaybeUriData<Arc<GPUBufferImage>>>>;
declare_component!(
  SceneTexture2dEntityDirectContent,
  SceneTexture2dEntity,
  TextureDirectContentType
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
  table: EntityComponentGroupTyped<T::Entity>,
) -> EntityComponentGroupTyped<T::Entity> {
  table
    .declare_foreign_key::<SceneTexture2dRefOf<T>>()
    .declare_foreign_key::<SceneSamplerRefOf<T>>()
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Texture2DWithSamplingDataView {
  pub texture: EntityHandle<SceneTexture2dEntity>,
  pub sampler: EntityHandle<SceneSamplerEntity>,
}

impl Texture2DWithSamplingDataView {
  pub fn read<T, E>(reader: &EntityReader<E>, id: EntityHandle<E>) -> Option<Self>
  where
    T: TextureWithSamplingForeignKeys<Entity = E>,
    E: EntitySemantic,
  {
    reader
      .read_foreign_key::<SceneTexture2dRefOf<T>>(id)
      .zip(reader.read_foreign_key::<SceneSamplerRefOf<T>>(id))
      .map(|(t, s)| Texture2DWithSamplingDataView {
        texture: t,
        sampler: s,
      })
  }
}

impl Texture2DWithSamplingDataView {
  pub fn write<C>(self, writer: EntityInitWriteView) -> EntityInitWriteView
  where
    C: TextureWithSamplingForeignKeys,
  {
    writer
      .write::<SceneTexture2dRefOf<C>>(&self.texture.some_handle())
      .write::<SceneSamplerRefOf<C>>(&self.sampler.some_handle())
  }
}

pub struct TexSamplerWriter<'a> {
  pub tex_writer: &'a mut EntityWriter<SceneTexture2dEntity>,
  pub sampler_writer: &'a mut EntityWriter<SceneSamplerEntity>,
}

impl TexSamplerWriter<'_> {
  pub fn write_direct_tex_with_default_sampler(
    &mut self,
    texture: GPUBufferImage,
  ) -> Texture2DWithSamplingDataView {
    let texture = Arc::new(texture);
    let texture = MaybeUriData::Living(texture);
    self.write_tex_with_default_sampler(texture)
  }

  pub fn write_tex_with_default_sampler(
    &mut self,
    texture: MaybeUriData<Arc<GPUBufferImage>>,
  ) -> Texture2DWithSamplingDataView {
    self.write(texture, TextureSampler::tri_linear_repeat())
  }

  pub fn write(
    &mut self,
    texture: MaybeUriData<Arc<GPUBufferImage>>,
    sampler: TextureSampler,
  ) -> Texture2DWithSamplingDataView {
    let sampler = self
      .sampler_writer
      .new_entity(|w| w.write::<SceneSamplerInfo>(&sampler));

    let texture = ExternalRefPtr::new(texture);
    let texture = self
      .tex_writer
      .new_entity(|w| w.write::<SceneTexture2dEntityDirectContent>(&texture.into()));

    Texture2DWithSamplingDataView { texture, sampler }
  }
}

pub struct TexCubeWriter<'a> {
  pub tex_writer: &'a mut EntityWriter<SceneTexture2dEntity>,
  pub cube_writer: &'a mut EntityWriter<SceneTextureCubeEntity>,
}

impl TexCubeWriter<'_> {
  pub fn write_cube_tex(
    &mut self,
    x_pos: GPUBufferImage,
    y_pos: GPUBufferImage,
    z_pos: GPUBufferImage,
    x_neg: GPUBufferImage,
    y_neg: GPUBufferImage,
    z_neg: GPUBufferImage,
  ) -> EntityHandle<SceneTextureCubeEntity> {
    let x_pos = self.tex_writer.new_entity(|w| {
      let x_pos = Arc::new(x_pos);
      let t = MaybeUriData::Living(x_pos);
      w.write::<SceneTexture2dEntityDirectContent>(&ExternalRefPtr::new(t).into())
    });
    let y_pos = self.tex_writer.new_entity(|w| {
      let y_pos = Arc::new(y_pos);
      let t = MaybeUriData::Living(y_pos);
      w.write::<SceneTexture2dEntityDirectContent>(&ExternalRefPtr::new(t).into())
    });
    let z_pos = self.tex_writer.new_entity(|w| {
      let z_pos = Arc::new(z_pos);
      let t = MaybeUriData::Living(z_pos);
      w.write::<SceneTexture2dEntityDirectContent>(&ExternalRefPtr::new(t).into())
    });
    let x_neg = self.tex_writer.new_entity(|w| {
      let x_neg = Arc::new(x_neg);
      let t = MaybeUriData::Living(x_neg);
      w.write::<SceneTexture2dEntityDirectContent>(&ExternalRefPtr::new(t).into())
    });
    let y_neg = self.tex_writer.new_entity(|w| {
      let y_neg = Arc::new(y_neg);
      let t = MaybeUriData::Living(y_neg);
      w.write::<SceneTexture2dEntityDirectContent>(&ExternalRefPtr::new(t).into())
    });
    let z_neg = self.tex_writer.new_entity(|w| {
      let z_neg = Arc::new(z_neg);
      let t = MaybeUriData::Living(z_neg);
      w.write::<SceneTexture2dEntityDirectContent>(&ExternalRefPtr::new(t).into())
    });

    self.cube_writer.new_entity(|w| {
      w.write::<SceneTextureCubeXPositiveFace>(&x_pos.some_handle())
        .write::<SceneTextureCubeYPositiveFace>(&y_pos.some_handle())
        .write::<SceneTextureCubeZPositiveFace>(&z_pos.some_handle())
        .write::<SceneTextureCubeXNegativeFace>(&x_neg.some_handle())
        .write::<SceneTextureCubeYNegativeFace>(&y_neg.some_handle())
        .write::<SceneTextureCubeZNegativeFace>(&z_neg.some_handle())
    })
  }
}
