pub use unlit_material::*;

use crate::*;

#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum SceneMaterialDataView {
  UnlitMaterial(EntityHandle<UnlitMaterialEntity>),
  PbrSGMaterial(EntityHandle<PbrSGMaterialEntity>),
  PbrMRMaterial(EntityHandle<PbrMRMaterialEntity>),
}

mod unlit_material {
  use crate::*;
  declare_entity!(UnlitMaterialEntity);
  declare_component!(
    UnlitMaterialColorComponent, // srgb
    UnlitMaterialEntity,
    Vec4<f32>,
    Vec4::one()
  );
  declare_entity_associated!(UnlitMaterialColorAlphaTex, UnlitMaterialEntity);
  impl TextureWithSamplingForeignKeys for UnlitMaterialColorAlphaTex {}
  pub fn register_unlit_material_data_model() {
    let table = global_database()
      .declare_entity::<UnlitMaterialEntity>()
      .declare_component::<UnlitMaterialColorComponent>();

    let table = register_texture_with_sampling::<UnlitMaterialColorAlphaTex>(table);

    register_alpha_config::<UnlitMaterialAlphaConfig>(table);
  }

  declare_entity_associated!(UnlitMaterialAlphaConfig, UnlitMaterialEntity);
  impl AlphaInfoSemantic for UnlitMaterialAlphaConfig {}

  #[derive(Default)]
  pub struct UnlitMaterialDataView {
    pub color: Vec4<f32>,
    pub color_alpha_tex: Option<Texture2DWithSamplingDataView>,
    pub alpha: AlphaConfigDataView,
  }
  impl EntityCustomWrite<UnlitMaterialEntity> for UnlitMaterialDataView {
    type Writer = EntityWriter<UnlitMaterialEntity>;

    fn create_writer() -> Self::Writer {
      global_entity_of::<UnlitMaterialEntity>().entity_writer()
    }

    fn write(self, writer: &mut Self::Writer) -> EntityHandle<UnlitMaterialEntity> {
      writer.new_entity(|w| {
        let w = w.write::<UnlitMaterialColorComponent>(&self.color);
        self.alpha.write::<UnlitMaterialAlphaConfig>(w)
      })
    }
  }
}

pub use sg_material::*;
mod sg_material {
  use crate::*;
  declare_entity!(PbrSGMaterialEntity);
  declare_component!(
    PbrSGMaterialAlbedoComponent, // linear
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

  declare_entity_associated!(PbrSGMaterialAlphaConfig, PbrSGMaterialEntity);
  impl AlphaInfoSemantic for PbrSGMaterialAlphaConfig {}

  declare_entity_associated!(PbrSGMaterialAlbedoAlphaTex, PbrSGMaterialEntity);
  impl TextureWithSamplingForeignKeys for PbrSGMaterialAlbedoAlphaTex {}
  declare_entity_associated!(PbrSGMaterialSpecularGlossinessTex, PbrSGMaterialEntity);
  impl TextureWithSamplingForeignKeys for PbrSGMaterialSpecularGlossinessTex {}
  declare_entity_associated!(PbrSGMaterialEmissiveTex, PbrSGMaterialEntity);
  impl TextureWithSamplingForeignKeys for PbrSGMaterialEmissiveTex {}
  declare_entity_associated!(PbrSGMaterialNormalInfo, PbrSGMaterialEntity);
  impl NormalInfoSemantic for PbrSGMaterialNormalInfo {}

  pub fn register_pbr_sg_material_data_model() {
    let table = global_database()
      .declare_entity::<PbrSGMaterialEntity>()
      .declare_component::<PbrSGMaterialAlbedoComponent>()
      .declare_component::<PbrSGMaterialSpecularComponent>()
      .declare_component::<PbrSGMaterialGlossinessComponent>()
      .declare_component::<PbrSGMaterialEmissiveComponent>();

    let table = register_texture_with_sampling::<PbrSGMaterialAlbedoAlphaTex>(table);
    let table = register_texture_with_sampling::<PbrSGMaterialSpecularGlossinessTex>(table);
    let table = register_texture_with_sampling::<PbrSGMaterialEmissiveTex>(table);
    let table = register_normal::<PbrSGMaterialNormalInfo>(table);
    register_alpha_config::<PbrSGMaterialAlphaConfig>(table);
  }

  #[derive(Clone)]
  pub struct PhysicalSpecularGlossinessMaterialDataView {
    pub albedo: Vec3<f32>,
    pub specular: Vec3<f32>,
    pub glossiness: f32,
    pub emissive: Vec3<f32>,
    pub alpha: AlphaConfigDataView,
    pub albedo_texture: Option<Texture2DWithSamplingDataView>,
    pub specular_glossiness_texture: Option<Texture2DWithSamplingDataView>,
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
        alpha: Default::default(),
        albedo_texture: None,
        specular_glossiness_texture: None,
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
      writer.new_entity(|w| {
        let w = w
          .write::<PbrSGMaterialAlbedoComponent>(&self.albedo)
          .write::<PbrSGMaterialSpecularComponent>(&self.specular)
          .write::<PbrSGMaterialGlossinessComponent>(&self.glossiness)
          .write::<PbrSGMaterialEmissiveComponent>(&self.emissive);

        let mut w = self.alpha.write::<PbrSGMaterialAlphaConfig>(w);

        if let Some(t) = self.albedo_texture {
          w = t.write::<PbrSGMaterialAlbedoAlphaTex>(w)
        }

        if let Some(t) = self.specular_glossiness_texture {
          w = t.write::<PbrSGMaterialSpecularGlossinessTex>(w)
        }

        if let Some(t) = self.emissive_texture {
          w = t.write::<PbrSGMaterialEmissiveTex>(w)
        }

        if let Some(normal) = self.normal_texture {
          w = normal.write::<PbrSGMaterialNormalInfo>(w);
        }

        w
      })
    }
  }
}

pub use mr_material::*;
mod mr_material {
  use crate::*;
  declare_entity!(PbrMRMaterialEntity);
  declare_component!(
    PbrMRMaterialBaseColorComponent, // linear
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

  declare_entity_associated!(PbrMRMaterialAlphaConfig, PbrMRMaterialEntity);
  impl AlphaInfoSemantic for PbrMRMaterialAlphaConfig {}

  declare_entity_associated!(PbrMRMaterialBaseColorAlphaTex, PbrMRMaterialEntity);
  impl TextureWithSamplingForeignKeys for PbrMRMaterialBaseColorAlphaTex {}
  declare_entity_associated!(PbrMRMaterialMetallicRoughnessTex, PbrMRMaterialEntity);
  impl TextureWithSamplingForeignKeys for PbrMRMaterialMetallicRoughnessTex {}
  declare_entity_associated!(PbrMRMaterialEmissiveTex, PbrMRMaterialEntity);
  impl TextureWithSamplingForeignKeys for PbrMRMaterialEmissiveTex {}
  declare_entity_associated!(PbrMRMaterialNormalInfo, PbrMRMaterialEntity);
  impl NormalInfoSemantic for PbrMRMaterialNormalInfo {}

  pub fn register_pbr_mr_material_data_model() {
    let table = global_database()
      .declare_entity::<PbrMRMaterialEntity>()
      .declare_component::<PbrMRMaterialBaseColorComponent>()
      .declare_component::<PbrMRMaterialRoughnessComponent>()
      .declare_component::<PbrMRMaterialMetallicComponent>()
      .declare_component::<PbrMRMaterialEmissiveComponent>();

    let table = register_texture_with_sampling::<PbrMRMaterialBaseColorAlphaTex>(table);
    let table = register_texture_with_sampling::<PbrMRMaterialMetallicRoughnessTex>(table);
    let table = register_texture_with_sampling::<PbrMRMaterialEmissiveTex>(table);
    let table = register_normal::<PbrMRMaterialNormalInfo>(table);
    register_alpha_config::<PbrMRMaterialAlphaConfig>(table);
  }

  #[derive(Clone)]
  pub struct PhysicalMetallicRoughnessMaterialDataView {
    /// in conductor case will act as specular color,
    /// in dielectric case will act as diffuse color,
    /// which is decided by metallic property
    pub base_color: Vec3<f32>,
    pub roughness: f32,
    pub metallic: f32,
    // pub reflectance: f32,
    pub emissive: Vec3<f32>,
    pub alpha: AlphaConfigDataView,
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
        alpha: Default::default(),
        emissive: Vec3::zero(),
        base_color_texture: None,
        metallic_roughness_texture: None,
        emissive_texture: None,
        // reflectance: 0.5,
        normal_texture: None,
      }
    }
  }

  impl PhysicalMetallicRoughnessMaterialDataView {
    pub fn write(
      self,
      writer: &mut EntityWriter<PbrMRMaterialEntity>,
    ) -> EntityHandle<PbrMRMaterialEntity> {
      writer.new_entity(|w| {
        let w = w
          .write::<PbrMRMaterialBaseColorComponent>(&self.base_color)
          .write::<PbrMRMaterialRoughnessComponent>(&self.roughness)
          .write::<PbrMRMaterialMetallicComponent>(&self.metallic)
          .write::<PbrMRMaterialEmissiveComponent>(&self.emissive);

        let mut w = self.alpha.write::<PbrMRMaterialAlphaConfig>(w);

        if let Some(t) = self.base_color_texture {
          w = t.write::<PbrMRMaterialBaseColorAlphaTex>(w);
        }

        if let Some(t) = self.metallic_roughness_texture {
          w = t.write::<PbrMRMaterialMetallicRoughnessTex>(w);
        }

        if let Some(t) = self.emissive_texture {
          w = t.write::<PbrMRMaterialEmissiveTex>(w);
        }

        if let Some(normal) = self.normal_texture {
          w = normal.write::<PbrMRMaterialNormalInfo>(w);
        }
        w
      })
    }
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
  table: EntityComponentGroupTyped<T::Entity>,
) -> EntityComponentGroupTyped<T::Entity> {
  let table = register_texture_with_sampling::<NormalTexSamplerOf<T>>(table);
  table.declare_component::<NormalScaleOf<T>>()
}

#[derive(Clone)]
pub struct NormalMappingDataView {
  pub content: Texture2DWithSamplingDataView,
  pub scale: f32,
}

impl NormalMappingDataView {
  pub fn write<C>(self, writer: EntityInitWriteView) -> EntityInitWriteView
  where
    C: NormalInfoSemantic,
  {
    let w = self.content.write::<NormalTexSamplerOf<C>>(writer);
    w.write::<NormalScaleOf<C>>(&self.scale)
  }

  pub fn read<T, E>(reader: &EntityReader<E>, id: EntityHandle<E>) -> Option<Self>
  where
    T: NormalInfoSemantic<Entity = E>,
    E: EntitySemantic,
  {
    Texture2DWithSamplingDataView::read::<NormalTexSamplerOf<T>, _>(reader, id).map(|t| {
      NormalMappingDataView {
        content: t,
        scale: reader.read::<NormalScaleOf<T>>(id),
      }
    })
  }
}

#[derive(Clone, Copy)]
pub struct AlphaConfigDataView {
  pub alpha_mode: AlphaMode,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}

impl Default for AlphaConfigDataView {
  fn default() -> Self {
    Self {
      alpha_mode: Default::default(),
      alpha_cutoff: 0.5,
      alpha: 1.0,
    }
  }
}

impl AlphaConfigDataView {
  pub fn write<C>(self, writer: EntityInitWriteView) -> EntityInitWriteView
  where
    C: AlphaInfoSemantic,
  {
    writer
      .write::<AlphaCutoffOf<C>>(&self.alpha_cutoff)
      .write::<AlphaModeOf<C>>(&self.alpha_mode)
      .write::<AlphaOf<C>>(&self.alpha)
  }

  pub fn read<T, E>(reader: &EntityReader<E>, id: EntityHandle<E>) -> Self
  where
    T: AlphaInfoSemantic<Entity = E>,
    E: EntitySemantic,
  {
    AlphaConfigDataView {
      alpha_mode: reader.read::<AlphaModeOf<T>>(id),
      alpha_cutoff: reader.read::<AlphaCutoffOf<T>>(id),
      alpha: reader.read::<AlphaOf<T>>(id),
    }
  }
}

pub trait AlphaInfoSemantic: EntityAssociateSemantic {}
pub struct AlphaOf<T>(T);
impl<T: AlphaInfoSemantic> EntityAssociateSemantic for AlphaOf<T> {
  type Entity = T::Entity;
}
impl<T: AlphaInfoSemantic> ComponentSemantic for AlphaOf<T> {
  type Data = f32;
  fn default_override() -> Self::Data {
    1.0
  }
}

pub struct AlphaCutoffOf<T>(T);
impl<T: AlphaInfoSemantic> EntityAssociateSemantic for AlphaCutoffOf<T> {
  type Entity = T::Entity;
}
impl<T: AlphaInfoSemantic> ComponentSemantic for AlphaCutoffOf<T> {
  type Data = f32;
  fn default_override() -> Self::Data {
    0.5
  }
}
pub struct AlphaModeOf<T>(T);
impl<T: AlphaInfoSemantic> EntityAssociateSemantic for AlphaModeOf<T> {
  type Entity = T::Entity;
}
impl<T: AlphaInfoSemantic> ComponentSemantic for AlphaModeOf<T> {
  type Data = AlphaMode;
  fn default_override() -> Self::Data {
    AlphaMode::Opaque
  }
}

pub fn register_alpha_config<T: AlphaInfoSemantic>(
  table: EntityComponentGroupTyped<T::Entity>,
) -> EntityComponentGroupTyped<T::Entity> {
  table
    .declare_component::<AlphaOf<T>>()
    .declare_component::<AlphaModeOf<T>>()
    .declare_component::<AlphaCutoffOf<T>>()
}

/// The alpha rendering mode of a material.
#[repr(C)]
#[derive(Serialize, Deserialize)]
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug, Facet)]
#[derive(Default)]
pub enum AlphaMode {
  /// The alpha value is ignored and the rendered output is fully opaque.
  #[default]
  Opaque,

  /// The rendered output is either fully opaque or fully transparent depending on
  /// the alpha value and the specified alpha cutoff value.
  Mask,

  /// The alpha value is used, to determine the transparency of the rendered output.
  /// The alpha cutoff value is ignored.
  Blend,
}
